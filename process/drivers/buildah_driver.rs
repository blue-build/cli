use blue_build_utils::{credentials::Credentials, secret::SecretArgs, semver::Version};
use colored::Colorize;
use comlexr::{cmd, pipe};
use log::{debug, error, info, trace, warn};
use miette::{Context, IntoDiagnostic, Result, bail};
use serde::Deserialize;
use tempfile::TempDir;

use crate::{
    drivers::opts::{ManifestCreateOpts, ManifestPushOpts},
    logging::CommandLogging,
};

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

        let temp_dir = TempDir::new()
            .into_diagnostic()
            .wrap_err("Failed to create temporary directory for secrets")?;

        let command = cmd!(
            "buildah",
            "build",
            for opts.secrets.args(&temp_dir)?,
            if opts.secrets.ssh() => "--ssh",
            if let Some(platform) = opts.platform => [
                "--platform",
                platform.to_string(),
            ],
            "--pull=true",
            format!("--layers={}", !opts.squash),
            match opts.cache_from.as_ref() {
                Some(cache_from) if !opts.squash => [
                    "--cache-from",
                    format!(
                        "{}/{}",
                        cache_from.registry(),
                        cache_from.repository()
                    ),
                ],
                _ => [],
            },
            match opts.cache_from.as_ref() {
                Some(cache_to) if !opts.squash => [
                    "--cache-to",
                    format!(
                        "{}/{}",
                        cache_to.registry(),
                        cache_to.repository()
                    ),
                ],
                _ => [],
            },
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

    fn login(server: &str) -> Result<()> {
        trace!("BuildahDriver::login()");

        if let Some(Credentials::Basic { username, password }) = Credentials::get(server) {
            let output = pipe!(
                stdin = password.value();
                {
                    let c = cmd!(
                        "buildah",
                        "login",
                        "-u",
                        &username,
                        "--password-stdin",
                        server,
                    );
                    trace!("{c:?}");
                    c
                }
            )
            .output()
            .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for buildah:\n{}", err_out.trim());
            }
            debug!("Logged into {server}");
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

    fn manifest_create(opts: ManifestCreateOpts) -> Result<()> {
        let output = {
            let c = cmd!("buildah", "manifest", "rm", opts.final_image.to_string());
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if output.status.success() {
            warn!(
                "Existing image manifest {} exists, removing...",
                opts.final_image
            );
        }

        let output = {
            let c = cmd!(
                "buildah",
                "manifest",
                "create",
                opts.final_image.to_string(),
                for image in opts.image_list => format!("containers-storage:{image}"),
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!(
                "Failed to create manifest for {}:\n{}",
                opts.final_image,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn manifest_push(opts: ManifestPushOpts) -> Result<()> {
        let image = &opts.final_image.to_string();
        let status = {
            let c = cmd!(
                "buildah",
                "manifest",
                "push",
                "--all",
                image,
                format!("docker://{}", opts.final_image),
            );
            trace!("{c:?}");
            c
        }
        .build_status(image, format!("Pushing manifest {image}..."))
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to create manifest for {}", opts.final_image);
        }

        Ok(())
    }
}
