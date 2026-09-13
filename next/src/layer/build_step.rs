#![expect(refining_impl_trait_internal)]
mod container;
mod states {
    /// When the build is ready to begin.
    pub struct ReadyState;

    /// When the build is running layered.
    pub struct RunningLayeredState;

    /// When the build is running squashed.
    pub struct RunningSquashedState;

    /// Trait used by running states.
    pub trait RunningState {}
    impl RunningState for RunningLayeredState {}
    impl RunningState for RunningSquashedState {}
}

use std::{hash::Hash, marker::PhantomData};

use blue_build_utils::platform::Platform;
use miette::Result;
use states::{ReadyState, RunningLayeredState, RunningSquashedState, RunningState};

use crate::layer::{BuildLayer, FromLayerReference, Layer, LayerId};

pub use container::Container;

/// Actions that can be taken on a `BuildStep`
/// that is in a `ReadyState`.
pub(super) trait StepBuilder {
    /// Runs the build step by
    ///
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn StepBuilder>>;

    /// Finalizes the build returning a `LayerId` that
    /// can be used for a mount, stage, or another build.
    fn finalize(self: Box<Self>) -> Result<LayerId>;
}

/// Commit the `BuildStep`.
trait StepCommiter {
    /// Commits the `Container` and returns a `Buildable` `BuildStep`.
    fn commit(self, container: Container) -> Result<impl StepBuilder>;
}

/// Create a `Container` from a ready `BuildStep`.
trait StepContainer {
    /// Create the main `Container` for the `BuildStep`
    /// for the layer. Returns an empty `BuildStep` in a `RunningState`.
    ///
    /// Use the returned `BuildStep` to commit the `Container`.
    fn container(self) -> Result<(Container, BuildStep<(), impl RunningState>)>;
}

/// A type-based state machine used
/// to track layer and container references
/// while performing build operations.
///
/// The state machine aspect of this type
/// prevents the build from being misused
/// by making a `Layer` retrieve a `Container`
/// from it (via `CreateContainer`), then requires
/// passing it back in to commit (via `Commiter`).
#[derive(Debug)]
pub struct BuildStep<Base, State> {
    base: Base,
    _state: PhantomData<State>,
}

/// A `BuildStep` for layer based builds.
pub type LayeredStep = BuildStep<LayerId, ReadyState>;

/// A `BuildStep` for squashed builds.
pub type SquashedStep = BuildStep<Container, ReadyState>;

/// A `BuildStep` in a `RunningState` for layer based builds.
pub type RunningLayeredStep = BuildStep<(), RunningLayeredState>;

/// A `BuildStep` in a `RunningState` for squashed builds.
pub type RunningSquashedStep = BuildStep<(), RunningSquashedState>;

#[bon::bon]
impl SquashedStep {
    #[builder(finish_fn = "build")]
    pub fn new_squashed(
        image: impl AsRef<FromLayerReference>,
        platform: Option<Platform>,
    ) -> Result<Self> {
        Ok(Self {
            base: Container::from_image()
                .image(image.as_ref())
                .maybe_platform(platform)
                .build()?,
            _state: PhantomData,
        })
    }
}

impl From<Box<Self>> for SquashedStep {
    fn from(value: Box<Self>) -> Self {
        Self {
            base: value.base,
            _state: PhantomData,
        }
    }
}

#[bon::bon]
impl LayeredStep {
    #[builder(finish_fn = "build")]
    pub fn new_layered(
        image: impl AsRef<FromLayerReference>,
        platform: Option<Platform>,
    ) -> Result<Self> {
        Ok(Self {
            base: LayerId::builder()
                .container(
                    Container::from_image()
                        .image(image.as_ref())
                        .maybe_platform(platform)
                        .build()?,
                )
                .build()?,
            _state: PhantomData,
        })
    }
}

impl From<Box<Self>> for LayeredStep {
    fn from(value: Box<Self>) -> Self {
        Self {
            base: value.base,
            _state: PhantomData,
        }
    }
}

impl Clone for LayeredStep {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            _state: PhantomData,
        }
    }
}

impl Hash for LayeredStep {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
    }
}

impl PartialEq for LayeredStep {
    fn eq(&self, other: &Self) -> bool {
        self.base.eq(&other.base)
    }
}

impl Eq for LayeredStep {}

impl StepContainer for LayeredStep {
    fn container(self) -> Result<(Container, BuildStep<(), RunningLayeredState>)> {
        Ok((
            Container::from_layer(&self.base)?,
            BuildStep {
                base: (),
                _state: PhantomData,
            },
        ))
    }
}

impl StepContainer for SquashedStep {
    fn container(self) -> Result<(Container, BuildStep<(), RunningSquashedState>)> {
        Ok((
            self.base,
            BuildStep {
                base: (),
                _state: PhantomData::<RunningSquashedState>,
            },
        ))
    }
}

impl StepCommiter for RunningLayeredStep {
    fn commit(self, container: Container) -> Result<LayeredStep> {
        Ok(BuildStep {
            base: LayerId::builder().container(container).build()?,
            _state: PhantomData,
        })
    }
}

impl StepCommiter for RunningSquashedStep {
    fn commit(self, container: Container) -> Result<SquashedStep> {
        Ok(BuildStep {
            base: container,
            _state: PhantomData,
        })
    }
}

impl StepBuilder for LayeredStep {
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn StepBuilder>> {
        #[cached::cached(sync_writes = "by_key", key = "Layer", convert = "{ layer.clone() }")]
        fn inner(step: LayeredStep, layer: &Layer) -> Result<LayeredStep> {
            let (container, step) = step.container()?;
            layer.run(&container)?;
            step.commit(container)
        }
        Ok(Box::new(inner(self.into(), &layer)?))
    }

    fn finalize(self: Box<Self>) -> Result<LayerId> {
        Ok(self.base)
    }
}

impl StepBuilder for SquashedStep {
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn StepBuilder>> {
        let (container, step) = self.container()?;
        layer.run(&container)?;
        Ok(Box::new(step.commit(container)?))
    }

    fn finalize(self: Box<Self>) -> Result<LayerId> {
        LayerId::builder().container(self.base).build()
    }
}
