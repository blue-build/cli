use bollard_buildkit_proto::moby::buildkit::v1::control_client::ControlClient;
use miette::{IntoDiagnostic, Result};
use tonic::transport::Channel;

use super::{opts::RunOpts, BuildDriver, Driver, RunDriver};

pub struct BuildkitDriver;

impl BuildkitDriver {
    /// Sets up buildkit container for building images.
    ///
    /// # Errors
    /// Errors if the buildkit container can't be created.
    #[allow(clippy::missing_panics_doc)]
    pub fn setup() -> Result<()> {
        Driver::create_container(
            &RunOpts::builder()
                .image(&"moby/buildkit".try_into().expect("Valid image"))
                .command("buildkit")
                .build(),
        )?;

        Ok(())
    }
}

impl BuildDriver for BuildkitDriver {
    fn build(_opts: &super::opts::BuildOpts) -> Result<()> {
        unimplemented!()
    }

    fn tag(_opts: &super::opts::TagOpts) -> Result<()> {
        unimplemented!()
    }

    fn push(_opts: &super::opts::PushOpts) -> Result<()> {
        unimplemented!()
    }

    fn login() -> Result<()> {
        todo!()
    }

    fn prune(_opts: &super::opts::PruneOpts) -> Result<()> {
        todo!()
    }

    fn build_tag_push(_opts: &super::opts::BuildTagPushOpts) -> Result<Vec<String>> {
        let client = ControlClient::new(Channel::builder(
            "localhost:8371".try_into().into_diagnostic()?,
        ));
        todo!()
    }
}
