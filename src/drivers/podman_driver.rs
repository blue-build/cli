use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use blue_build_utils::constants::SKOPEO_IMAGE;
use log::{debug, info, trace};
use semver::Version;
use serde::Deserialize;

use crate::image_metadata::ImageMetadata;

use super::{credentials, opts::CompressionType, BuildDriver, DriverVersion, InspectDriver};

#[derive(Debug, Deserialize)]
struct PodmanVersionJsonClient {
    #[serde(alias = "Version")]
    pub version: Version,
}

#[derive(Debug, Deserialize)]
struct PodmanVersionJson {
    #[serde(alias = "Client")]
    pub client: PodmanVersionJsonClient,
}

#[derive(Debug)]
pub struct PodmanDriver;

impl DriverVersion for PodmanDriver {
    // First podman version to use buildah v1.24
    // https://github.com/containers/podman/blob/main/RELEASE_NOTES.md#400
    const VERSION_REQ: &'static str = ">=4";

    fn version() -> Result<Version> {
        let output = Command::new("podman")
            .arg("version")
            .arg("-f")
            .arg("json")
            .output()?;

        let version_json: PodmanVersionJson = serde_json::from_slice(&output.stdout)?;

        Ok(version_json.client.version)
    }
}

impl BuildDriver for PodmanDriver {
    fn build(&self, image: &str) -> Result<()> {
        trace!("podman build . -t {image}");
        let status = Command::new("podman")
            .arg("build")
            .arg(".")
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

        trace!("podman tag {src_image} {dest_image}");
        let status = Command::new("podman")
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

    fn push(&self, image: &str, compression: CompressionType) -> Result<()> {
        trace!("podman push {image}");
        let status = Command::new("podman")
            .arg("push")
            .arg(format!("--compression-format={compression}"))
            .arg(image)
            .status()?;

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

        trace!("podman login -u {username} -p [MASKED] {registry}");
        let output = Command::new("podman")
            .arg("login")
            .arg("-u")
            .arg(username)
            .arg("-p")
            .arg(password)
            .arg(registry)
            .output()?;

        if !output.status.success() {
            let err_out = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to login for podman: {err_out}");
        }
        Ok(())
    }
}

impl InspectDriver for PodmanDriver {
    fn get_metadata(&self, image_name: &str, tag: &str) -> Result<ImageMetadata> {
        let url = format!("docker://{image_name}:{tag}");

        trace!("podman run {SKOPEO_IMAGE} inspect {url}");
        let output = Command::new("podman")
            .arg("run")
            .arg(SKOPEO_IMAGE)
            .arg("inspect")
            .arg(&url)
            .stderr(Stdio::inherit())
            .output()?;

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
