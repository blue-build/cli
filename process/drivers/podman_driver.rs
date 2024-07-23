use std::{
    path::Path,
    process::{Command, ExitStatus},
    time::Duration,
};

use blue_build_utils::{cmd, constants::SKOPEO_IMAGE};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, trace, warn};
use miette::{bail, IntoDiagnostic, Result};
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    credentials::Credentials,
    drivers::image_metadata::ImageMetadata,
    logging::{CommandLogging, Logger},
    signal_handler::{add_cid, remove_cid, ContainerId, ContainerRuntime},
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
        let output = cmd!("podman", "version", "-f", "json")
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

        let command = cmd!(
            "podman",
            "build",
            "--pull=true",
            format!("--layers={}", !opts.squash),
            "-f",
            opts.containerfile.as_ref(),
            "-t",
            opts.image.as_ref(),
            ".",
        );

        trace!("{command:?}");
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

        let mut command = cmd!(
            "podman",
            "tag",
            opts.src_image.as_ref(),
            opts.dest_image.as_ref(),
        );

        trace!("{command:?}");
        let status = command.status().into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(opts: &PushOpts) -> Result<()> {
        trace!("PodmanDriver::push({opts:#?})");

        let command = cmd!(
            "podman",
            "push",
            format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ),
            opts.image.as_ref(),
        );

        trace!("{command:?}");
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
            let output = cmd!("podman", "login", "-u", username, "-p", password, registry)
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
                .args(["inspect".to_string(), url.clone()])
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

        let cid = ContainerId::new(&cid_file, ContainerRuntime::Podman, opts.privileged);

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

        let cid = ContainerId::new(&cid_file, ContainerRuntime::Podman, opts.privileged);

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
        cmd!("sudo")
    } else {
        cmd!("podman")
    };

    if opts.privileged {
        cmd!(command, "podman");
    }

    cmd!(command, "run", format!("--cidfile={}", cid_file.display()));

    if opts.privileged {
        cmd!(command, "--privileged");
    }

    if opts.remove {
        cmd!(command, "--rm");
    }

    if opts.pull {
        cmd!(command, "--pull=always");
    }

    opts.volumes.iter().for_each(|volume| {
        cmd!(
            command,
            "--volume",
            format!("{}:{}", volume.path_or_vol_name, volume.container_path,)
        );
    });

    opts.env_vars.iter().for_each(|env| {
        cmd!(command, "--env", format!("{}={}", env.key, env.value));
    });

    cmd!(command, opts.image.as_ref());

    opts.args.iter().for_each(|arg| cmd!(command, arg));

    trace!("{command:?}");
    command
}
