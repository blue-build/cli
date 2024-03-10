use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Result};
use log::{info, trace};

use crate::strategies::ENV_CREDENTIALS;

use super::{BuildStrategy, InspectStrategy};

#[derive(Debug)]
pub struct BuildahStrategy;

impl BuildStrategy for BuildahStrategy {
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
        let (registry, username, password) = ENV_CREDENTIALS
            .as_ref()
            .map(|credentials| {
                (
                    &credentials.registry,
                    &credentials.username,
                    &credentials.password,
                )
            })
            .ok_or_else(|| anyhow!("Unable to login, missing credentials!"))?;

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

impl InspectStrategy for BuildahStrategy {
    fn get_labels(
        &self,
        image_name: &str,
        tag: &str,
    ) -> Result<crate::image_inspection::ImageInspection> {
        let skopeo_url = "docker://quay.io/skopeo/stable:latest".to_string();
        let url = format!("docker://{image_name}:{tag}");

        trace!("buildah run {skopeo_url} inspect {url}");
        let output = Command::new("buildah")
            .arg("run")
            .arg(skopeo_url)
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
