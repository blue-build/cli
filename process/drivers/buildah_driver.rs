use blue_build_utils::cmd;
use log::{error, info, trace};
use miette::{bail, IntoDiagnostic, Result};
use semver::Version;
use serde::Deserialize;

use crate::{credentials::Credentials, logging::CommandLogging};

use super::{
    opts::{BuildOpts, PushOpts, TagOpts},
    BuildDriver, DriverVersion,
};

#[derive(Debug, Deserialize)]
struct BuildahVersionJson {
    pub version: Version,
}

#[derive(Debug)]
pub struct BuildahDriver;

impl DriverVersion for BuildahDriver {
    // RUN mounts for bind, cache, and tmpfs first supported in 1.24.0
    // https://buildah.io/releases/#changes-for-v1240
    const VERSION_REQ: &'static str = ">=1.24";

    fn version() -> Result<Version> {
        trace!("BuildahDriver::version()");

        trace!("buildah version --json");
        let output = cmd!("buildah", "version", "--json")
            .output()
            .into_diagnostic()?;

        let version_json: BuildahVersionJson = serde_json::from_slice(&output.stdout)
            .inspect_err(|e| error!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
            .into_diagnostic()?;
        trace!("{version_json:#?}");

        Ok(version_json.version)
    }
}

impl BuildDriver for BuildahDriver {
    fn build(opts: &BuildOpts) -> Result<()> {
        trace!("BuildahDriver::build({opts:#?})");

        let command = cmd!(
            "buildah",
            "build",
            "--pull=true",
            format!("--layers={}", !opts.squash),
            "-f",
            opts.containerfile.as_ref(),
            "-t",
            opts.image.as_ref(),
        );

        trace!("{command:?}");
        let status = command
            .status_image_ref_progress(&opts.image, "Building Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(opts: &TagOpts) -> Result<()> {
        trace!("BuildahDriver::tag({opts:#?})");

        let mut command = cmd!(
            "buildah",
            "tag",
            opts.src_image.as_ref(),
            opts.dest_image.as_ref(),
        );

        trace!("{command:?}");
        if command.status().into_diagnostic()?.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(opts: &PushOpts) -> Result<()> {
        trace!("BuildahDriver::push({opts:#?})");

        let command = cmd!(
            "buildah",
            "push",
            format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ),
            opts.image.as_ref(),
        );

        trace!("{command:?}");
        let status = command
            .status_image_ref_progress(&opts.image, "Pushing Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image);
        }
        Ok(())
    }

    fn login() -> Result<()> {
        trace!("BuildahDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            trace!("buildah login -u {username} -p [MASKED] {registry}");
            let output = cmd!("buildah", "login", "-u", username, "-p", password, registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for buildah: {err_out}");
            }
        }
        Ok(())
    }
}
