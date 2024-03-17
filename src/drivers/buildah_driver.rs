use std::process::Command;

use anyhow::{bail, Result};
use log::{info, trace};
use semver::Version;
use serde::Deserialize;

use crate::credentials;

use super::{BuildDriver, DriverVersion};

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
        let output = Command::new("buildah")
            .arg("version")
            .arg("--json")
            .output()?;

        let version_json: BuildahVersionJson = serde_json::from_slice(&output.stdout)?;

        Ok(version_json.version)
    }
}

impl BuildDriver for BuildahDriver {
    fn build(&self, image: &str) -> Result<()> {
        trace!("buildah build -t {image}");
        let status = Command::new("buildah")
            .arg("build")
            .arg("-t")
            .arg(image)
            .status()?;

        if status.success() {
            info!("Successfully built {image}");
        } else {
            bail!("Failed to build {image}");
        }
        Ok(())
    }

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()> {
        let dest_image = format!("{image_name}:{tag}");
        trace!("buildah tag {src_image} {dest_image}");
        let status = Command::new("buildah")
            .arg("tag")
            .arg(src_image)
            .arg(&dest_image)
            .status()?;

        if status.success() {
            info!("Successfully tagged {dest_image}!");
        } else {
            bail!("Failed to tag image {dest_image}");
        }
        Ok(())
    }

    fn push(&self, image: &str) -> Result<()> {
        trace!("buildah push {image}");
        let status = Command::new("buildah").arg("push").arg(image).status()?;

        if status.success() {
            info!("Successfully pushed {image}!");
        } else {
            bail!("Failed to push image {image}")
        }
        Ok(())
    }

    fn login(&self) -> Result<()> {
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
