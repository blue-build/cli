use std::sync::Arc;

use blue_build_utils::platform::Platform;
use miette::Result;

use crate::{
    containers::Container,
    layer::{
        LayerId,
        build_step::{BuildLayer, BuildStep, Buildable, Layered},
    },
    reference::PinnedReference,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum FromLayerReference {
    #[default]
    Scratch,
    Image(Arc<PinnedReference>),
    Layer(Arc<LayerId>),
}

impl From<Arc<PinnedReference>> for FromLayerReference {
    fn from(value: Arc<PinnedReference>) -> Self {
        Self::Image(value)
    }
}

impl From<Arc<LayerId>> for FromLayerReference {
    fn from(value: Arc<LayerId>) -> Self {
        Self::Layer(value)
    }
}

impl std::fmt::Display for FromLayerReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Scratch => "scratch".to_string(),
                Self::Image(image) => image.to_string(),
                Self::Layer(layer) => layer.to_string(),
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FromLayer {
    from: FromLayerReference,
    platform: Platform,
    squash: bool,
}

#[bon::bon]
impl FromLayer {
    #[builder(derive(Into))]
    pub fn new(
        #[builder(into, default)] from: FromLayerReference,
        #[builder(default)] platform: Platform,
        #[builder(default)] squash: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            from,
            platform,
            squash,
        })
    }
}

impl BuildLayer for Arc<FromLayer> {
    fn run_step(&self, _container: &Container) -> Result<()> {
        unimplemented!()
    }

    fn build(&self) -> Result<Box<dyn Buildable>> {
        #[cached::cached(
            sync_writes = "by_key",
            convert = "{ layer.clone() }",
            key = "Arc<FromLayer>"
        )]
        fn inner(layer: &Arc<FromLayer>) -> Result<Layered> {
            let container = Container::from_image()
                .image(&layer.from)
                .platform(layer.platform)
                .build()?;
            dbg!(&container);

            BuildStep::new_layered(container)
        }

        Ok(if self.squash {
            Box::new(BuildStep::new_squashed(
                Container::from_image()
                    .image(&self.from)
                    .platform(self.platform)
                    .build()?,
            ))
        } else {
            Box::new(inner(self)?)
        })
    }
}

#[cfg(test)]
mod test {

    use blue_build_utils::platform::Platform;
    use oci_client::Reference;
    use rstest::rstest;

    use crate::{
        layer::{FinalizeLayer, from::FromLayer},
        reference::PinnedReference,
    };

    #[rstest]
    #[case("quay.io/fedora/fedora")]
    #[case("docker.io/library/alpine")]
    fn eval(#[case] image: &str) {
        let image: &Reference = &image.parse().unwrap();
        let image = PinnedReference::pin_image(image).unwrap();
        let platform = Platform::default();

        let layer = FromLayer::builder().from(image).platform(platform).build();

        dbg!(&layer);

        let id = layer.finalize();

        dbg!(&id);

        assert!(id.is_ok());
    }
}
