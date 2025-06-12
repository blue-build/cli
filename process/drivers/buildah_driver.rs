use std::{io::Write, process::Stdio};

use blue_build_utils::{credentials::Credentials, semver::Version};
use colored::Colorize;
use comlexr::cmd;
use log::{debug, error, info, trace};
use miette::{IntoDiagnostic, Result, bail, miette};
use serde::Deserialize;

use crate::{drivers::types::Platform, logging::CommandLogging};

use super::{
    BuildDriver, DriverVersion,
    opts::{BuildOpts, PruneOpts, PushOpts, TagOpts},
};

#[derive(Debug, Deserialize)]
struct BuildahVersionJson {
    pub version: Version,
}

#[derive(Debug)]
pub struct BuildahDriver;

impl DriverVersion for BuildahDriver {
    // The prune command wasn't present until 1.29
    const VERSION_REQ: &'static str = ">=1.29";

    fn version() -> Result<Version> {
        trace!("BuildahDriver::version()");

        let output = {
            let c = cmd!("buildah", "version", "--json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        let version_json: BuildahVersionJson = serde_json::from_slice(&output.stdout)
            .inspect_err(|e| error!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
            .into_diagnostic()?;
        trace!("{version_json:#?}");

        Ok(version_json.version)
    }
}

impl BuildDriver for BuildahDriver {
    fn build(opts: BuildOpts) -> Result<()> {
        trace!("BuildahDriver::build({opts:#?})");

        let command = cmd!(
            "buildah",
            "build",
            if !matches!(opts.platform, Platform::Native) => [
                "--platform",
                opts.platform.to_string(),
            ],
            "--pull=true",
            format!("--layers={}", !opts.squash),
            if let Some(cache_from) = opts.cache_from.as_ref() => [
                "--cache-from",
                format!(
                    "{}/{}",
                    cache_from.registry(),
                    cache_from.repository()
                ),
            ],
            if let Some(cache_to) = opts.cache_to.as_ref() => [
                "--cache-to",
                format!(
                    "{}/{}",
                    cache_to.registry(),
                    cache_to.repository()
                ),
            ],
            "-f",
            opts.containerfile,
            "-t",
            opts.image.to_string(),
        );

        trace!("{command:?}");
        let status = command
            .build_status(opts.image.to_string(), "Building Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(opts: TagOpts) -> Result<()> {
        trace!("BuildahDriver::tag({opts:#?})");

        let dest_image_str = opts.dest_image.to_string();

        let mut command = cmd!(
            "buildah",
            "tag",
            opts.src_image.to_string(),
            &dest_image_str,
        );

        trace!("{command:?}");
        if command.status().into_diagnostic()?.success() {
            info!("Successfully tagged {}!", dest_image_str.bold().green());
        } else {
            bail!("Failed to tag image {}", dest_image_str.bold().red());
        }
        Ok(())
    }

    fn push(opts: PushOpts) -> Result<()> {
        trace!("BuildahDriver::push({opts:#?})");

        let image_str = opts.image.to_string();

        let command = cmd!(
            "buildah",
            "push",
            format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ),
            &image_str,
        );

        trace!("{command:?}");
        let status = command
            .build_status(&image_str, "Pushing Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", image_str.bold().green());
        } else {
            bail!("Failed to push image {}", image_str.bold().red());
        }
        Ok(())
    }

    fn login() -> Result<()> {
        trace!("BuildahDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            let mut command = cmd!(
                "buildah",
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
                bail!("Failed to login for buildah:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }
        Ok(())
    }

    fn prune(opts: PruneOpts) -> Result<()> {
        trace!("PodmanDriver::prune({opts:?})");

        let status = cmd!(
            "buildah",
            "prune",
            "--force",
            if opts.all => "--all",
        )
        .message_status("buildah prune", "Pruning Buildah System")
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to prune buildah");
        }

        Ok(())
    }
}
