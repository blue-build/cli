use std::{process::Command, time::Duration};

use anyhow::{bail, Result};
use blue_build_utils::{
    constants::SKOPEO_IMAGE,
    logging::{CommandLogging, Logger},
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, trace};
use semver::Version;
use serde::Deserialize;

use crate::{credentials::Credentials, image_metadata::ImageMetadata};

use super::{
    credentials,
    opts::{BuildOpts, GetMetadataOpts, PushOpts, TagOpts},
    BuildDriver, DriverVersion, InspectDriver,
};

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
        trace!("PodmanDriver::version()");

        trace!("podman version -f json");
        let output = Command::new("podman")
            .arg("version")
            .arg("-f")
            .arg("json")
            .output()?;

        let version_json: PodmanVersionJson = serde_json::from_slice(&output.stdout)
            .inspect_err(|e| error!("{e}: {}", String::from_utf8_lossy(&output.stdout)))?;
        trace!("{version_json:#?}");

        Ok(version_json.client.version)
    }
}

impl BuildDriver for PodmanDriver {
    fn build(&self, opts: &BuildOpts) -> Result<()> {
        trace!("PodmanDriver::build({opts:#?})");

        trace!(
            "podman build --pull=true --layers={} -f {} -t {} .",
            !opts.squash,
            opts.containerfile.display(),
            opts.image,
        );
        let mut command = Command::new("podman");
        command
            .arg("build")
            .arg("--pull=true")
            .arg(format!("--layers={}", !opts.squash))
            .arg("-f")
            .arg(opts.containerfile.as_ref())
            .arg("-t")
            .arg(opts.image.as_ref())
            .arg(".");
        let status = command.status_image_ref_progress(&opts.image, "Building Image")?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(&self, opts: &TagOpts) -> Result<()> {
        trace!("PodmanDriver::tag({opts:#?})");

        trace!("podman tag {} {}", opts.src_image, opts.dest_image);
        let status = Command::new("podman")
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
        trace!("PodmanDriver::push({opts:#?})");

        trace!("podman push {}", opts.image);
        let mut command = Command::new("podman");
        command
            .arg("push")
            .arg(format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ))
            .arg(opts.image.as_ref());
        let status = command.status_image_ref_progress(&opts.image, "Pushing Image")?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image)
        }
        Ok(())
    }

    fn login(&self) -> Result<()> {
        trace!("PodmanDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = credentials::get()
        {
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
        }
        Ok(())
    }
}

impl InspectDriver for PodmanDriver {
    fn get_metadata(&self, opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("PodmanDriver::get_metadata({opts:#?})");

        let url = opts.tag.as_ref().map_or_else(
            || format!("docker://{}", opts.image),
            |tag| format!("docker://{}:{tag}", opts.image),
        );

        let progress = Logger::multi_progress().add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner())
                .with_message(format!("Inspecting metadata for {url}")),
        );
        progress.enable_steady_tick(Duration::from_millis(100));

        trace!("podman run {SKOPEO_IMAGE} inspect {url}");
        let output = Command::new("podman")
            .arg("run")
            .arg("--rm")
            .arg(SKOPEO_IMAGE)
            .arg("inspect")
            .arg(&url)
            .output()?;

        progress.finish();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}");
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
