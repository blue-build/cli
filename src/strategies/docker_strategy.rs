use std::{
    env,
    process::{Command, Stdio},
};

use anyhow::{bail, Result};
use blue_build_utils::constants::*;
use log::{info, trace};

use crate::{credentials, image_inspection::ImageInspection};

use super::{BuildStrategy, InspectStrategy};

#[derive(Debug)]
pub struct DockerStrategy;

impl BuildStrategy for DockerStrategy {
    fn build(&self, image: &str) -> Result<()> {
        let docker_help = Command::new("docker")
            .arg("build")
            .arg("--help")
            .output()?
            .stdout;
        let docker_help = String::from_utf8_lossy(&docker_help);

        trace!("docker");
        let mut command = Command::new("docker");

        if docker_help.lines().filter(|l| l.contains("buildx")).count() > 0 {
            trace!("buildx build --load");
            command.arg("buildx").arg("build").arg("--load");
        } else {
            trace!("build");
            command.arg("build");
        }

        // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
        if env::var(BB_BUILDKIT_CACHE_GHA).map_or_else(|_| false, |e| e == "true") {
            trace!("--cache-from type=gha --cache-to type=gha");
            command
                .arg("--cache-from")
                .arg("type=gha")
                .arg("--cache-to")
                .arg("type=gha");
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
        let (registry, username, password) = credentials::get_credentials().map(|credentials| {
            (
                &credentials.registry,
                &credentials.username,
                &credentials.password,
            )
        })?;

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

impl InspectStrategy for DockerStrategy {
    fn get_labels(&self, image_name: &str, tag: &str) -> Result<ImageInspection> {
        let skopeo_url = "quay.io/skopeo/stable:latest".to_string();
        let url = format!("docker://{image_name}:{tag}");

        trace!("docker run {skopeo_url} inspect {url}");
        let output = Command::new("docker")
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
