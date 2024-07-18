use std::{
    path::Path,
    process::{Command, ExitStatus},
    time::Duration,
};

use blue_build_utils::constants::SKOPEO_IMAGE;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, trace, warn};
use miette::{bail, IntoDiagnostic, Result};
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    credentials::Credentials,
    drivers::{image_metadata::ImageMetadata, types::RunDriverType},
    logging::{CommandLogging, Logger},
    signal_handler::{add_cid, remove_cid, ContainerId},
};

use super::{
    opts::{BuildOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
    BuildDriver, DriverVersion, InspectDriver, RunDriver,
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
            .output()
            .into_diagnostic()?;

        let version_json: PodmanVersionJson = serde_json::from_slice(&output.stdout)
            .inspect_err(|e| error!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
            .into_diagnostic()?;
        trace!("{version_json:#?}");

        Ok(version_json.client.version)
    }
}

impl BuildDriver for PodmanDriver {
    fn build(opts: &BuildOpts) -> Result<()> {
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
        trace!("PodmanDriver::tag({opts:#?})");

        trace!("podman tag {} {}", opts.src_image, opts.dest_image);
        let status = Command::new("podman")
            .arg("tag")
            .arg(opts.src_image.as_ref())
            .arg(opts.dest_image.as_ref())
            .status()
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(opts: &PushOpts) -> Result<()> {
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
        let status = command
            .status_image_ref_progress(&opts.image, "Pushing Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image)
        }
        Ok(())
    }

    fn login() -> Result<()> {
        trace!("PodmanDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            trace!("podman login -u {username} -p [MASKED] {registry}");
            let output = Command::new("podman")
                .arg("login")
                .arg("-u")
                .arg(username)
                .arg("-p")
                .arg(password)
                .arg(registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for podman: {err_out}");
            }
        }
        Ok(())
    }
}

impl InspectDriver for PodmanDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
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

        let output = Self::run_output(
            &RunOpts::builder()
                .image(SKOPEO_IMAGE)
                .args(["inspect", url.as_str()])
                .remove(true)
                .build(),
        )
        .into_diagnostic()?;

        progress.finish();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}");
        }
        serde_json::from_slice(&output.stdout).into_diagnostic()
    }
}

impl RunDriver for PodmanDriver {
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus> {
        trace!("PodmanDriver::run({opts:#?})");

        let cid_path = TempDir::new("podman")?;
        let cid_file = cid_path.path().join("cid");

        let cid = ContainerId::new(&cid_file, RunDriverType::Podman, opts.privileged);

        add_cid(&cid);

        let status = podman_run(opts, &cid_file)
            .status_image_ref_progress(opts.image.as_ref(), "Running container")?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: &RunOpts) -> std::io::Result<std::process::Output> {
        trace!("PodmanDriver::run_output({opts:#?})");

        let cid_path = TempDir::new("podman")?;
        let cid_file = cid_path.path().join("cid");

        let cid = ContainerId::new(&cid_file, RunDriverType::Podman, opts.privileged);

        add_cid(&cid);

        let output = podman_run(opts, &cid_file).output()?;

        remove_cid(&cid);

        Ok(output)
    }
}

fn podman_run(opts: &RunOpts, cid_file: &Path) -> Command {
    let mut command = if opts.privileged {
        warn!(
            "Running 'podman' in privileged mode requires '{}'",
            "sudo".bold().red()
        );
        Command::new("sudo")
    } else {
        Command::new("podman")
    };

    if opts.privileged {
        command.arg("podman");
    }

    command
        .arg("run")
        .arg(format!("--cidfile={}", cid_file.display()));

    if opts.privileged {
        command.arg("--privileged");
    }

    if opts.remove {
        command.arg("--rm");
    }

    if opts.pull {
        command.arg("--pull=always");
    }

    opts.volumes.iter().for_each(|volume| {
        command.arg("--volume");
        command.arg(format!(
            "{}:{}",
            volume.path_or_vol_name, volume.container_path,
        ));
    });

    opts.env_vars.iter().for_each(|env| {
        command.arg("--env");
        command.arg(format!("{}={}", env.key, env.value));
    });

    command.arg(opts.image.as_ref());
    command.args(opts.args.iter());

    trace!("{command:?}");
    command
}
