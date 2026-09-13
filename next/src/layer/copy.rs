use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use blue_build_utils::cmd_out;
use miette::{IntoDiagnostic, Result};

use crate::{
    layer::{
        BuildLayer, FinalizeLayer, Layer,
        build_step::{Container, ReadyBuildStep, StepBuilder},
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
    async fn run(&self, container: &Container) -> Result<()> {
        let from = match &self.from {
            None => None,
            Some(layer) => Some(layer.clone().finalize().await?),
        };
        let container = container.as_str().to_owned();
        let source_path = self.source.path().to_owned();
        let source_context = self.source.context().to_owned();
        let destination = self.destination.clone();
        tokio::task::spawn_blocking(move || {
            cmd_out!(
                err_msg = format!(
                    "Failed to copy files {}from path {} to path {}",
                    from.as_ref().map_or_default(|from| format!("from container {from} ")),
                    source_path.display(),
                    destination.display(),
                );
                "buildah",
                "copy",
                format!("--contextdir={}", source_context.display()),
                if let Some(layer_id) = &from => format!("--from={layer_id}"),
                &container,
                &source_path,
                &destination,
            )
        })
        .await
        .into_diagnostic()?
    }

    async fn build(&self) -> Result<ReadyBuildStep> {
        self.parent
            .build()
            .await?
            .run_build_step(self.clone().into())
            .await
    }
}
