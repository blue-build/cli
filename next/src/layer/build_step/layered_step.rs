use std::{hash::Hash, marker::PhantomData};

use blue_build_utils::platform::Platform;
use miette::Result;

use crate::layer::{
    BuildLayer, FromLayerReference, Layer, LayerId, StepBuilder,
    build_step::{
        BuildStep, Container, ReadyBuildStep, StepCommiter, StepContainer,
        states::{ReadyState, RunningLayeredState},
    },
};

/// A `BuildStep` for layer based builds.
pub type LayeredStep = BuildStep<LayerId, ReadyState>;

/// A `BuildStep` in a `RunningState` for layer based builds.
pub type RunningLayeredStep = BuildStep<(), RunningLayeredState>;

#[bon::bon]
impl LayeredStep {
    #[builder(finish_fn = "build")]
    pub async fn new_layered(
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
                .build()
                .await?,
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
    async fn container(self) -> Result<(Container, RunningLayeredStep)> {
        Ok((
            Container::from_layer(&self.base).await?,
            BuildStep {
                base: (),
                _state: PhantomData,
            },
        ))
    }
}

impl StepCommiter for RunningLayeredStep {
    async fn commit(self, container: Container) -> Result<LayeredStep> {
        Ok(BuildStep {
            base: LayerId::builder().container(container).build().await?,
            _state: PhantomData,
        })
    }
}

impl StepBuilder for LayeredStep {
    async fn run_build_step(self, layer: Layer) -> Result<ReadyBuildStep> {
        #[cached::cached(sync_writes = "by_key", key = "Layer", convert = "{ layer.clone() }")]
        async fn inner(step: LayeredStep, layer: &Layer) -> Result<LayeredStep> {
            let (container, step) = step.container().await?;
            layer.run(&container).await?;
            let step = step.commit(container).await?;

            Ok(step)
        }
        Ok(inner(self, &layer).await?.into())
    }

    async fn finalize(self) -> Result<LayerId> {
        async move { Ok(self.base) }.await
    }
}
