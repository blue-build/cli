use std::{
    env,
    process::{Command, Stdio},
};

use anyhow::{bail, Result};
use blue_build_utils::constants::{BB_BUILDKIT_CACHE_GHA, SKOPEO_IMAGE};
use log::{info, trace};
use semver::Version;
use serde::Deserialize;

use crate::image_inspection::ImageInspection;

use super::{credentials, BuildDriver, DriverVersion, InspectDriver};

#[derive(Debug, Deserialize)]
struct DockerVerisonJsonClient {
    #[serde(alias = "Version")]
    pub version: Version,
}

#[derive(Debug, Deserialize)]
struct DockerVersionJson {
    #[serde(alias = "Client")]
    pub client: DockerVerisonJsonClient,
}

#[derive(Debug)]
pub struct DockerDriver;

impl DriverVersion for DockerDriver {
    // First docker verison to use buildkit
    // https://docs.docker.com/build/buildkit/
    const VERSION_REQ: &'static str = ">=23";

    fn version() -> Result<Version> {
        let output = Command::new("docker")
            .arg("version")
            .arg("-f")
            .arg("json")
            .output()?;

        let version_json: DockerVersionJson = serde_json::from_slice(&output.stdout)?;

        Ok(version_json.client.version)
    }
}

impl BuildDriver for DockerDriver {
    fn build(&self, image: &str) -> Result<()> {
        trace!("docker");
        let mut command = Command::new("docker");

        // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
        if env::var(BB_BUILDKIT_CACHE_GHA).map_or_else(|_| false, |e| e == "true") {
            trace!("buildx build --load --cache-from type=gha --cache-to type=gha");
            command
                .arg("buildx")
                .arg("build")
                .arg("--load")
                .arg("--cache-from")
                .arg("type=gha")
                .arg("--cache-to")
                .arg("type=gha");
        } else {
            trace!("build");
            command.arg("build");
        }

        trace!("-t {image} -f Containerfile .");
        command
            .arg("-t")
            .arg(image)
            .arg("-f")
            .arg("Containerfile")
            .arg(".");

        if command.status()?.success() {
            info!("Successfully built {image}");
        } else {
            bail!("Failed to build {image}");
        }
        Ok(())
    }

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()> {
        let dest_image = format!("{image_name}:{tag}");

        trace!("docker tag {src_image} {dest_image}");
        let status = Command::new("docker")
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
        trace!("docker push {image}");
        let status = Command::new("docker").arg("push").arg(image).status()?;

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

        trace!("docker login -u {username} -p [MASKED] {registry}");
        let output = Command::new("docker")
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

impl InspectDriver for DockerDriver {
    fn get_labels(&self, image_name: &str, tag: &str) -> Result<ImageInspection> {
        let url = format!("docker://{image_name}:{tag}");

        trace!("docker run {SKOPEO_IMAGE} inspect {url}");
        let output = Command::new("docker")
            .arg("run")
            .arg(SKOPEO_IMAGE)
            .arg("inspect")
            .arg(&url)
            .stderr(Stdio::inherit())
            .output()?;

        if output.status.success() {
            info!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
