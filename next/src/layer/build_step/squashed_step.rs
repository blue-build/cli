use std::marker::PhantomData;

use blue_build_utils::platform::Platform;
use miette::Result;

use crate::layer::{
    BuildLayer, FromLayerReference, Layer, LayerId, StepBuilder,
    build_step::{
        BuildStep, Container, ReadyBuildStep, StepCommiter, StepContainer,
        states::{ReadyState, RunningSquashedState},
    },
};

/// A `BuildStep` for squashed builds.
pub type SquashedStep = BuildStep<Container, ReadyState>;

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

impl StepContainer for SquashedStep {
    async fn container(self) -> Result<(Container, RunningSquashedStep)> {
        async move {
            Ok((
                self.base,
                BuildStep {
                    base: (),
                    _state: PhantomData,
                },
            ))
        }
        .await
    }
}

impl StepCommiter for RunningSquashedStep {
    async fn commit(self, container: Container) -> Result<SquashedStep> {
        async move {
            Ok(BuildStep {
                base: container,
                _state: PhantomData,
            })
        }
        .await
    }
}

impl StepBuilder for SquashedStep {
    async fn run_build_step(self, layer: Layer) -> Result<ReadyBuildStep> {
        let (container, step) = self.container().await?;
        layer.run(&container).await?;
        Ok(step.commit(container).await?.into())
    }

    async fn finalize(self) -> Result<LayerId> {
        LayerId::builder().container(self.base).build().await
    }
}
