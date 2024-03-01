use std::{env, process::Command};

use anyhow::{anyhow, bail, Result};
use blue_build_utils::constants::*;
use log::{info, trace};
use typed_builder::TypedBuilder;

use crate::commands::build::Credentials;

use super::BuildStrategy;

#[derive(Debug, TypedBuilder)]
pub struct DockerStrategy {
    creds: Option<Credentials>,
}

impl BuildStrategy for DockerStrategy {
    fn build(&self, image: &str) -> Result<()> {
        trace!("docker build . -t {image}");
        let mut command = Command::new("docker");
        command
            .arg("build")
            .arg(".")
            .arg("-t")
            .arg(image)
            .arg("-f")
            .arg("Containerfile");

        // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
        if env::var(BB_BUILDKIT_CACHE_GHA)
            .map_or_else(|_| false, |e| e.parse::<bool>().unwrap_or_default())
        {
            trace!("--cache-from type=gha --cache-to type=gha");
            command
                .arg("--cache-from")
                .arg("type=gha")
                .arg("--cache-to")
                .arg("type=gha");
        }

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
        let (registry, username, password) = self
            .creds
            .as_ref()
            .map(|credentials| {
                (
                    &credentials.registry,
                    &credentials.username,
                    &credentials.password,
                )
            })
            .ok_or_else(|| anyhow!("Unable to login, missing credentials!"))?;

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
