use std::{collections::BTreeMap, process::Command, sync::Arc};

use blue_build_utils::cmd_out;
use miette::Result;

use crate::layer::{
    BuildLayer, Layer, LayerCommand, Mount,
    build_step::{Container, StepBuilder},
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum RunCommand {
    /// Sanitized command that helps
    /// with escape sequences.
    Sanitized(LayerCommand),

    /// Unsanitized command that is
    /// passed directly into the shell.
    Unsanitized(String),
}

impl From<Command> for RunCommand {
    fn from(value: Command) -> Self {
        Self::Sanitized(value.into())
    }
}

impl From<LayerCommand> for RunCommand {
    fn from(value: LayerCommand) -> Self {
        Self::Sanitized(value)
    }
}

impl From<String> for RunCommand {
    fn from(value: String) -> Self {
        Self::Unsanitized(value)
    }
}

impl From<&str> for RunCommand {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl std::fmt::Display for RunCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Sanitized(cmd) => cmd.to_string(),
                Self::Unsanitized(cmd) => cmd.clone(),
            }
        )
    }
}

/// A `Layer` that allows you to run a `Command`
/// in a build container.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RunLayer {
    parent: Layer,
    args: BTreeMap<String, String>,
    shell: LayerCommand,
    command: RunCommand,
    mounts: Vec<Mount>,
}

#[bon::bon]
impl RunLayer {
    /// Builder for a `RunLayer`
    #[builder(derive(Into))]
    pub fn new(
        /// The `Layer` to run a command on.
        #[builder(into)]
        parent: Layer,

        /// The equivalent of `ARG` for this single instruction.
        #[builder(default, with = FromIterator::from_iter)]
        args: BTreeMap<String, String>,

        /// The `Command` to run in the container.
        #[builder(into)]
        command: RunCommand,

        /// The shell `Command` to use for the run.
        ///
        /// The `cmd` will be passed in-full as a
        /// single arg after all shell args.
        #[builder(
            into,
            default = LayerCommand::from(
                comlexr::cmd!("/bin/sh", "-c")
            )
        )]
        shell: LayerCommand,

        /// The `Mount`s to use during the run.
        #[builder(default, with = FromIterator::from_iter)]
        mounts: Vec<Mount>,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            args,
            shell,
            command,
            mounts,
        })
    }

    pub(super) fn parent(&self) -> Layer {
        self.parent.clone()
    }
}

impl BuildLayer for Arc<RunLayer> {
    fn run(&self, container: &Container) -> Result<()> {
        cmd_out!(
            err_msg = format!("Failed to run command\n{:?}", self.command);
            "buildah",
            "run",
            "--add-history",
            "--tty=false",
            format!("--env=TARGETARCH={}", self.parent().get_root().platform()),
            for (key, value) in &self.args => format!("--env={key}={value}"),
            for mount in &self.mounts => mount.finalize()?,
            &container,
            "--",
            &self.shell.program,
            for &self.shell.args,
            self.command.to_string(),
        )
    }

    fn build(&self) -> Result<Box<dyn StepBuilder>> {
        self.parent.build()?.run_build_step(self.clone().into())
    }
}

#[cfg(test)]
mod test {
    use std::process::Command;

    use comlexr::cmd;
    use rstest::rstest;

    use crate::{
        layer::{FinalizeLayer, from::FromLayer, run::RunLayer},
        reference::PinnedReference,
    };

    #[rstest]
    #[case(
        cmd!("touch", "/test")
    )]
    fn eval(#[case] cmd: Command) {
        let image = PinnedReference::try_from("quay.io/fedora/fedora").unwrap();
        let from_layer = FromLayer::builder().from(image).build();

        let layer = RunLayer::builder().parent(from_layer).command(cmd).build();

        dbg!(&layer);

        let id = layer.finalize();

        dbg!(&id);

        assert!(id.is_ok());
    }
}
