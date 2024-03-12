use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use blue_build_utils::constants::SKOPEO_IMAGE;
use log::{debug, info, trace};

use crate::image_inspection::ImageInspection;

use super::{credentials, BuildStrategy, InspectStrategy};

#[derive(Debug)]
pub struct PodmanStrategy;

impl BuildStrategy for PodmanStrategy {
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

    fn push(&self, image: &str) -> Result<()> {
        trace!("podman push {image}");
        let status = Command::new("podman").arg("push").arg(image).status()?;

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
            bail!("Failed to login for buildah: {err_out}");
        }
        Ok(())
    }
}

impl InspectStrategy for PodmanStrategy {
    fn get_labels(&self, image_name: &str, tag: &str) -> Result<ImageInspection> {
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
