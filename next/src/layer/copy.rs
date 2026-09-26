use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use blue_build_utils::cmd_out;
use miette::Result;

use crate::{
    layer::{
        BuildLayer, Layer,
        build_step::{Container, StepBuilder},
    },
    path::ContextPath,
};

/// A `Layer` for copying files into a build container.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CopyLayer {
    parent: Layer,
    from: Option<Layer>,
    source: ContextPath,
    destination: PathBuf,
}

#[bon::bon]
impl CopyLayer {
    /// Builder for `CopyLayer`.
    #[builder(derive(Into))]
    pub fn new(
        /// The parent `Layer` to copy files onto.
        #[builder(into)]
        parent: Layer,

        /// The `Layer` of a stage to copy files from.
        #[builder(into)]
        from: Option<Layer>,

        /// The source to copy from. You must supply
        /// a `NormalizedPath` and `RelativePath` to
        /// create a guraunteed path that resides
        /// within the build's context
        ///
        /// # Errors
        /// Will error if the `ContextPath` fails to
        /// resolve properly.
        #[builder(with = |context: impl AsRef<Path>, path: impl AsRef<Path>| -> Result<_> {
            Ok(ContextPath::builder()
                .path(path.as_ref())?
                .context_dir(context.as_ref())?
                .build())
        })]
        source: ContextPath,

        /// The location in the build to copy the files to.
        #[builder(into)]
        destination: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            from,
            source,
            destination,
        })
    }

    pub(super) fn parent(&self) -> Layer {
        self.parent.clone()
    }
}

impl BuildLayer for Arc<CopyLayer> {
    fn run(&self, container: &Container) -> Result<()> {
        let from = match &self.from {
            None => None,
            Some(layer) => Some(layer.build()?.finalize()?),
        };
        cmd_out!(
            err_msg = format!(
                "Failed to copy files {}from path {} to path {}",
                from.as_ref().map_or_default(|from| format!("from container {from} ")),
                self.source.path().display(),
                self.destination.display(),
            );
            "buildah",
            "copy",
            format!("--contextdir={}", self.source.context().display()),
            if let Some(layer_id) = &from => format!("--from={layer_id}"),
            &container,
            &self.source.path(),
            &self.destination,
        )?;
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn StepBuilder>> {
        self.parent.build()?.run_build_step(self.clone().into())
    }
}
