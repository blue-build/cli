#![expect(refining_impl_trait_internal)]
mod container;
mod layered_step;
mod squashed_step;
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

use std::marker::PhantomData;

use miette::Result;

use crate::layer::{
    Layer, LayerId,
    build_step::{squashed_step::SquashedStep, states::RunningState},
};

pub use container::Container;
pub use layered_step::LayeredStep;

/// Actions that can be taken on a `BuildStep`
/// that is in a `ReadyState`.
pub(super) trait StepBuilder {
    /// Runs the build step by
    ///
    async fn run_build_step(self, layer: Layer) -> Result<ReadyBuildStep>;

    /// Finalizes the build returning a `LayerId` that
    /// can be used for a mount, stage, or another build.
    async fn finalize(self) -> Result<LayerId>;
}

/// Commit the `BuildStep`.
trait StepCommiter {
    /// Commits the `Container` and returns a `Buildable` `BuildStep`.
    async fn commit(self, container: Container) -> Result<impl StepBuilder>;
}

/// Create a `Container` from a ready `BuildStep`.
trait StepContainer {
    /// Create the main `Container` for the `BuildStep`
    /// for the layer. Returns an empty `BuildStep` in a `RunningState`.
    ///
    /// Use the returned `BuildStep` to commit the `Container`.
    async fn container(self) -> Result<(Container, BuildStep<(), impl RunningState>)>;
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

pub enum ReadyBuildStep {
    Layer(LayeredStep),
    Squash(SquashedStep),
}

macro_rules! impl_ready {
    ($($var:ident => $typ:ty),* $(,)+) => {
        $(
        impl From<$typ> for ReadyBuildStep {
            fn from(value: $typ) -> Self {
                Self::$var(value)
            }
        }
        )*

        impl StepBuilder for ReadyBuildStep {
            async fn run_build_step(self, layer: Layer) -> Result<ReadyBuildStep> {
                match self {
                    $(Self::$var(val) => val.run_build_step(layer).await,)*
                }
            }

            async fn finalize(self) -> Result<LayerId> {
                match self {
                    $(Self::$var(val) => val.finalize().await,)*
                }
            }
        }
    };
}

impl_ready!(
    Layer => LayeredStep,
    Squash => SquashedStep,
);

// pub enum RunningBuildStep {
//     Layer(RunningLayeredStep),
//     Squash(RunningSquashedStep),
// }

// macro_rules! impl_running {
//     ($($var:ident => $typ:ty),* $(,)+) => {
//         $(
//         impl From<$typ> for RunningBuildStep {
//             fn from(value: $typ) -> Self {
//                 Self::$var(value)
//             }
//         }
//         )*

//         impl StepCommiter for RunningBuildStep {
//             async fn commit(self, container: Container) -> Result<ReadyBuildStep> {
//                 match self {
//                     $(Self::$var(val) => val.commit(container).await,)*
//                 }
//             }
//         }
//     };
// }

// impl_running!(
//     Layer => RunningLayeredStep,
//     Squash => RunningSquashedStep,
// );
