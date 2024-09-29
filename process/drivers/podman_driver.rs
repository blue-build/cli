use std::{
    io::Write,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    time::Duration,
};

use blue_build_utils::{cmd, constants::SKOPEO_IMAGE, credentials::Credentials};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, trace, warn};
use miette::{bail, miette, IntoDiagnostic, Result};
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    drivers::{
        image_metadata::ImageMetadata,
        opts::{RunOptsEnv, RunOptsVolume},
    },
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
            &*opts.containerfile,
            "-t",
            &*opts.image,
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

        let mut command = cmd!("podman", "tag", &*opts.src_image, &*opts.dest_image,);

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
            &*opts.image,
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
            let mut command = cmd!(
                "podman",
                "login",
                "-u",
                username,
                "--password-stdin",
                registry
            );
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            trace!("{command:?}");
            let mut child = command.spawn().into_diagnostic()?;

            write!(
                child
                    .stdin
                    .as_mut()
                    .ok_or_else(|| miette!("Unable to open pipe to stdin"))?,
                "{password}"
            )
            .into_diagnostic()?;

            let output = child.wait_with_output().into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for podman:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }
        Ok(())
    }
}

impl InspectDriver for PodmanDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("PodmanDriver::get_metadata({opts:#?})");

        let url = opts.tag.as_deref().map_or_else(
            || format!("docker://{}", opts.image),
            |tag| format!("docker://{}:{tag}", opts.image),
        );

        let progress = Logger::multi_progress().add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner())
                .with_message(format!("Inspecting metadata for {}", url.bold())),
        );
        progress.enable_steady_tick(Duration::from_millis(100));

        let output = Self::run_output(
            &RunOpts::builder()
                .image(SKOPEO_IMAGE)
                .args(bon::vec!["inspect", &url])
                .remove(true)
                .build(),
        )
        .into_diagnostic()?;

        progress.finish_and_clear();
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

        let status = if opts.privileged {
            podman_run(opts, &cid_file).status()?
        } else {
            podman_run(opts, &cid_file)
                .status_image_ref_progress(&*opts.image, "Running container")?
        };

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
    let command = cmd!(
        if opts.privileged {
            warn!(
                "Running 'podman' in privileged mode requires '{}'",
                "sudo".bold().red()
            );
            "sudo"
        } else {
            "podman"
        },
        if opts.privileged => "podman",
        "run",
        format!("--cidfile={}", cid_file.display()),
        if opts.privileged => [
            "--privileged",
            "--network=host",
        ],
        if opts.remove => "--rm",
        if opts.pull => "--pull=always",
        for RunOptsVolume { path_or_vol_name, container_path } in opts.volumes.iter() => [
            "--volume",
            format!("{path_or_vol_name}:{container_path}"),
        ],
        for RunOptsEnv { key, value } in opts.env_vars.iter() => [
            "--env",
            format!("{key}={value}"),
        ],
        &*opts.image,
        for arg in opts.args.iter() => &**arg,
    );
    trace!("{command:?}");

    command
}
