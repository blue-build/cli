#![expect(refining_impl_trait_internal)]
mod states {
    pub struct ReadyState;
    pub struct RunningLayeredState;
    pub struct RunningSquashedState;

    pub trait RunningState {}
    impl RunningState for RunningLayeredState {}
    impl RunningState for RunningSquashedState {}
}

use std::{hash::Hash, marker::PhantomData, sync::Arc};

use miette::Result;
use states::{ReadyState, RunningLayeredState, RunningSquashedState, RunningState};

use crate::{
    containers::Container,
    layer::{CopyLayer, FromLayer, Layer, LayerId, RunLayer},
};

pub trait Commiter {
    fn commit_step(self, container: Container) -> Result<impl Buildable>;
}

pub trait CreateContainer {
    fn container(self) -> Result<(Container, BuildStep<(), impl RunningState>)>;
}

pub trait Buildable {
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn Buildable>>;
    fn finalize(self: Box<Self>) -> Result<Arc<LayerId>>;
}

pub trait BuildLayer: Sized {
    fn run_step(&self, container: &Container) -> Result<()>;
    fn build(&self) -> Result<Box<dyn Buildable>>;
}

pub trait FinalizeLayer: BuildLayer {
    /// Finalize the layer by running the build graph
    /// and returning the `LayerId` that can be used
    /// to create another build graph.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn finalize(self) -> Result<Arc<LayerId>> {
        let step = self.build()?;
        step.finalize()
    }
}

impl FinalizeLayer for Arc<FromLayer> {}
impl FinalizeLayer for Arc<CopyLayer> {}
impl FinalizeLayer for Arc<RunLayer> {}

#[derive(Debug)]
pub struct BuildStep<Base, State> {
    base: Base,
    _state: PhantomData<State>,
}

pub type RunningSquashed = BuildStep<(), RunningSquashedState>;

pub type Squashed = BuildStep<Container, ReadyState>;

impl Squashed {
    pub const fn new_squashed(container: Container) -> Self {
        Self {
            base: container,
            _state: PhantomData,
        }
    }
}

impl From<Box<Self>> for Squashed {
    fn from(value: Box<Self>) -> Self {
        Self {
            base: value.base,
            _state: PhantomData,
        }
    }
}

pub type Layered = BuildStep<Arc<LayerId>, ReadyState>;

impl Layered {
    pub fn new_layered(container: Container) -> Result<Self> {
        Ok(Self {
            base: LayerId::builder().container(container).build()?,
            _state: PhantomData,
        })
    }
}

impl From<Box<Self>> for Layered {
    fn from(value: Box<Self>) -> Self {
        Self {
            base: value.base,
            _state: PhantomData,
        }
    }
}

impl Clone for Layered {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            _state: PhantomData,
        }
    }
}

impl Hash for Layered {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
    }
}

impl PartialEq for Layered {
    fn eq(&self, other: &Self) -> bool {
        self.base.eq(&other.base)
    }
}

impl Eq for Layered {}

impl CreateContainer for Layered {
    fn container(self) -> Result<(Container, BuildStep<(), RunningLayeredState>)> {
        Ok((
            Container::from_layer(&self.base)?,
            BuildStep {
                base: (),
                _state: PhantomData::<RunningLayeredState>,
            },
        ))
    }
}

impl CreateContainer for Squashed {
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

pub type RunningLayered = BuildStep<(), RunningLayeredState>;

impl Commiter for RunningLayered {
    fn commit_step(self, container: Container) -> Result<Layered> {
        Ok(BuildStep {
            base: LayerId::builder().container(container).build()?,
            _state: PhantomData,
        })
    }
}

impl Commiter for RunningSquashed {
    fn commit_step(self, container: Container) -> Result<Squashed> {
        Ok(BuildStep {
            base: container,
            _state: PhantomData,
        })
    }
}

impl Buildable for Layered {
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn Buildable>> {
        #[cached::cached(sync_writes = "by_key", key = "Layer", convert = "{ layer.clone() }")]
        fn inner(step: Layered, layer: &Layer) -> Result<Layered> {
            let (container, step) = step.container()?;
            layer.run_step(&container)?;
            step.commit_step(container)
        }
        Ok(Box::new(inner(self.into(), &layer)?))
    }

    fn finalize(self: Box<Self>) -> Result<Arc<LayerId>> {
        Ok(self.base)
    }
}

impl Buildable for Squashed {
    fn run_build_step(self: Box<Self>, layer: Layer) -> Result<Box<dyn Buildable>> {
        let (container, step) = self.container()?;
        layer.run_step(&container)?;
        Ok(Box::new(step.commit_step(container)?))
    }

    fn finalize(self: Box<Self>) -> Result<Arc<LayerId>> {
        LayerId::builder().container(self.base).build()
    }
}
