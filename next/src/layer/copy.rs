use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use blue_build_utils::cmd_out;
use miette::Result;

use crate::{
    layer::{
        Layer,
        build_step::{BuildLayer, Buildable},
    },
    path::ContextPath,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CopyLayer {
    parent: Layer,
    from: Option<Layer>,
    source: ContextPath,
    destination: PathBuf,
}

#[bon::bon]
impl CopyLayer {
    #[builder(derive(Into))]
    pub fn new(
        #[builder(into)] parent: Layer,

        #[builder(into)] from: Option<Layer>,

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

        #[builder(into)] destination: PathBuf,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            from,
            source,
            destination,
        })
    }
}

impl BuildLayer for Arc<CopyLayer> {
    fn run_step(&self, container: &crate::containers::Container) -> Result<()> {
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
            &container,
            format!("--contextdir={}", self.source.context().display()),
            if let Some(layer_id) = &from => format!("--from={layer_id}"),
            &self.source.path(),
            &self.destination,
        )?;
        Ok(())
    }

    fn build(&self) -> Result<Box<dyn Buildable>> {
        let step = self.parent.build()?;
        step.run_build_step(self.clone().into())
    }
}
