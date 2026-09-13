use std::{collections::BTreeMap, ffi::OsString, fmt::Display, process::Command, sync::Arc};

use blue_build_utils::cmd_out;
use comlexr::{cmd, cmd_mut};
use miette::Result;

use crate::{
    containers::Container,
    layer::{
        Layer, Mount,
        build_step::{BuildLayer, Buildable},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunLayerCommand {
    program: OsString,
    args: Vec<OsString>,
    envs: BTreeMap<OsString, OsString>,
}

impl From<Command> for RunLayerCommand {
    fn from(value: Command) -> Self {
        Self {
            program: value.get_program().to_owned(),
            args: value.get_args().map(ToOwned::to_owned).collect(),
            envs: value
                .get_envs()
                .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_owned())))
                .collect(),
        }
    }
}

impl From<&RunLayerCommand> for Command {
    fn from(value: &RunLayerCommand) -> Self {
        let mut c = cmd!(
            &value.program,
            for &value.args,
        );

        for (key, value) in &value.envs {
            cmd_mut!(
                env {
                    key: value,
                };
                &mut c,
            );
        }
        c
    }
}

impl Display for RunLayerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", Command::from(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunLayer {
    parent: Layer,
    cmd: RunLayerCommand,
    mounts: Vec<Mount>,
}

#[bon::bon]
impl RunLayer {
    #[builder(derive(Into))]
    pub fn new(
        #[builder(into)] parent: Layer,
        #[builder(into)] cmd: RunLayerCommand,
        #[builder(default, with = FromIterator::from_iter)] mounts: Vec<Mount>,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            cmd,
            mounts,
        })
    }
}

impl BuildLayer for Arc<RunLayer> {
    fn run_step(&self, container: &Container) -> Result<()> {
        cmd_out!(
            err_msg = format!("Failed to run command\n{:?}", self.cmd);
            "buildah",
            "run",
            "--add-history",
            "--tty=false",
            for mount in &self.mounts => mount.finalize()?,
            &container,
            "--",
            "/bin/sh",
            "-c",
            self.cmd.to_string(),
        )
    }

    fn build(&self) -> Result<Box<dyn Buildable>> {
        let step = self.parent.build()?;
        step.run_build_step(self.clone().into())
    }
}

#[cfg(test)]
mod test {
    use std::{process::Command, str::FromStr};

    use comlexr::cmd;
    use oci_client::Reference;
    use rstest::rstest;

    use crate::{
        layer::{build_step::FinalizeLayer, from::FromLayer, run::RunLayer},
        reference::PinnedReference,
    };

    #[rstest]
    #[case(
        cmd!("touch", "/test")
    )]
    fn eval(#[case] cmd: Command) {
        let image =
            PinnedReference::pin_image(&Reference::from_str("quay.io/fedora/fedora").unwrap())
                .unwrap();
        let from_layer = FromLayer::builder().from(image).build();

        let layer = RunLayer::builder().parent(from_layer).cmd(cmd).build();

        dbg!(&layer);

        let id = layer.finalize();

        dbg!(&id);

        assert!(id.is_ok());
    }
}
