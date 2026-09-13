use std::sync::Arc;

use blue_build_utils::cmd_out;

use crate::layer::{BuildLayer, Layer, LayerCommand};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct UnshareLayer {
    parent: Layer,
    commands: Vec<LayerCommand>,
}

#[bon::bon]
impl UnshareLayer {
    #[builder(derive(Into))]
    pub fn new(
        /// The `Layer` to mount and run host commands on.
        #[builder(into)]
        parent: Layer,

        /// The host commands to pass into `buildah unshare`.
        #[builder(with = FromIterator::from_iter)]
        commands: Vec<LayerCommand>,
    ) -> Arc<Self> {
        Arc::new(Self { parent, commands })
    }

    pub(super) fn parent(&self) -> Layer {
        self.parent.clone()
    }
}

impl BuildLayer for Arc<UnshareLayer> {
    fn run(&self, container: &super::build_step::Container) -> miette::Result<()> {
        let commands = self
            .commands
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(";\n");
        cmd_out!(
            err_msg = format!("Failed to run commands in unshare:\n{commands}");
            "buildah",
            "unshare",
            format!("--mount=BB_UNSHARE_MOUNT={container}"),
            "--",
            "/bin/sh",
            "-c",
            &commands
        )?;
        Ok(())
    }

    fn build(&self) -> miette::Result<Box<dyn super::StepBuilder>> {
        self.parent.build()?.run_build_step(self.clone().into())
    }
}
