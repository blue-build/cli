use std::sync::Arc;

use blue_build_utils::platform::Platform;
use miette::Result;

use crate::{
    layer::{
        BuildLayer, LayerId,
        build_step::{BuildStep, Container, LayeredStep, ReadyBuildStep},
    },
    reference::PinnedReference,
};

/// An enum to various types of
/// references that can be used to
/// start an image from.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum FromLayerReference {
    /// A scratch layer that has nothing in it.
    #[default]
    Scratch,

    /// A `PinnedReference` from an OCI or Docker registry.
    Image(PinnedReference),

    /// A `LayerId` from another build.
    Layer(LayerId),
}

impl From<PinnedReference> for FromLayerReference {
    fn from(value: PinnedReference) -> Self {
        Self::Image(value)
    }
}

impl From<LayerId> for FromLayerReference {
    fn from(value: LayerId) -> Self {
        Self::Layer(value)
    }
}

impl AsRef<Self> for FromLayerReference {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl std::fmt::Display for FromLayerReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Scratch => "scratch".to_string(),
                Self::Image(image) => format!("docker://{image}"),
                Self::Layer(layer) => layer.to_string(),
            }
        )
    }
}

/// The root `Layer` in the build graph.
///
/// This object is needed to start creating
/// a build graph. It's possible to start
/// from "scratch", an image, or another build.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FromLayer {
    from: FromLayerReference,
    platform: Option<Platform>,
    squash: bool,
}

#[bon::bon]
impl FromLayer {
    /// Builder for a `FromLayer`.
    #[builder(derive(Into))]
    pub fn new(
        /// The starting reference for the build.
        ///
        /// If not defined, starts the build on
        /// a "scratch" image.
        #[builder(into, default)]
        from: FromLayerReference,

        /// The platform to build for.
        ///
        /// Non-natvie platforms require emulation
        /// in order to run. If not defined, uses the
        /// native platform of the host.
        platform: Option<Platform>,

        /// Squash all changes of this build into
        /// a single layer.
        ///
        /// Prevents caching build steps, but performs
        /// better when building from inside a container.
        #[builder(default)]
        squash: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            from,
            platform,
            squash,
        })
    }

    pub(crate) fn platform(&self) -> Platform {
        self.platform.unwrap_or_default()
    }
}

impl BuildLayer for Arc<FromLayer> {
    async fn run(&self, _container: &Container) -> Result<()> {
        unimplemented!()
    }

    async fn build(&self) -> Result<ReadyBuildStep> {
        #[cached::cached(
            sync_writes = "by_key",
            convert = "{ layer.clone() }",
            key = "Arc<FromLayer>"
        )]
        async fn inner(layer: &Arc<FromLayer>) -> Result<LayeredStep> {
            BuildStep::new_layered()
                .image(&layer.from)
                .maybe_platform(layer.platform)
                .build()
                .await
        }

        Ok(if self.squash {
            BuildStep::new_squashed()
                .image(&self.from)
                .maybe_platform(self.platform)
                .build()?
                .into()
        } else {
            inner(self).await?.into()
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

    #[tokio::test]
    #[rstest]
    #[case("quay.io/fedora/fedora")]
    #[case("docker.io/library/alpine")]
    async fn eval(#[case] image: &str) {
        let image: Reference = image.parse().unwrap();
        let image = PinnedReference::pin_image(image).await.unwrap();
        let platform = Platform::default();

        let layer = FromLayer::builder().from(image).platform(platform).build();

        dbg!(&layer);

        let id = layer.finalize().await;

        dbg!(&id);

        assert!(id.is_ok());
    }
}
