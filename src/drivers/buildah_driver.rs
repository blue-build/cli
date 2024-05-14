use std::process::Command;

use anyhow::{bail, Result};
use blue_build_utils::logging::{shorten_image_names, CommandLogging};
use log::{info, trace};
use semver::Version;
use serde::Deserialize;

use crate::credentials;

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
        let output = Command::new("buildah")
            .arg("version")
            .arg("--json")
            .output()?;

        let version_json: BuildahVersionJson = serde_json::from_slice(&output.stdout)?;
        trace!("{version_json:#?}");

        Ok(version_json.version)
    }
}

impl BuildDriver for BuildahDriver {
    fn build(&self, opts: &BuildOpts) -> Result<()> {
        trace!("BuildahDriver::build({opts:#?})");

        trace!(
            "buildah build --pull=true --layers={} -f {} -t {}",
            !opts.squash,
            opts.containerfile.display(),
            opts.image,
        );
        let status = Command::new("buildah")
            .arg("build")
            .arg("--pull=true")
            .arg(format!("--layers={}", !opts.squash))
            .arg("-f")
            .arg(opts.containerfile.as_ref())
            .arg("-t")
            .arg(opts.image.as_ref())
            .status_log_prefix(&shorten_image_names(&opts.image))?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(&self, opts: &TagOpts) -> Result<()> {
        trace!("BuildahDriver::tag({opts:#?})");

        trace!("buildah tag {} {}", opts.src_image, opts.dest_image);
        let status = Command::new("buildah")
            .arg("tag")
            .arg(opts.src_image.as_ref())
            .arg(opts.dest_image.as_ref())
            .status()?;

        if status.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(&self, opts: &PushOpts) -> Result<()> {
        trace!("BuildahDriver::push({opts:#?})");

        trace!("buildah push {}", opts.image);
        let status = Command::new("buildah")
            .arg("push")
            .arg(format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ))
            .arg(opts.image.as_ref())
            .status_log_prefix(&format!("push - {}", shorten_image_names(&opts.image)))?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image);
        }
        Ok(())
    }

    fn login(&self) -> Result<()> {
        trace!("BuildahDriver::login()");

        let (registry, username, password) =
            credentials::get().map(|c| (&c.registry, &c.username, &c.password))?;

        trace!("buildah login -u {username} -p [MASKED] {registry}");
        let output = Command::new("buildah")
            .arg("login")
            .arg("-u")
            .arg(username)
            .arg("-p")
            .arg(password)
            .arg(registry)
            .output()?;

        if !output.status.success() {
            let err_out = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to login for buildah: {err_out}");
        }
        Ok(())
    }
}
