use std::{process::Stdio, time::Duration};

use cached::proc_macro::cached;
use colored::Colorize;
use comlexr::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, trace};
use miette::{IntoDiagnostic, Result, bail};

use crate::{drivers::types::Platform, logging::Logger};

use super::{
    InspectDriver,
    opts::{CopyOciDirOpts, GetMetadataOpts},
    types::ImageMetadata,
};

#[derive(Debug)]
pub struct SkopeoDriver;

impl InspectDriver for SkopeoDriver {
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata> {
        get_metadata_cache(opts)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ format!("{}-{}", opts.image, opts.platform)}"#,
    sync_writes = "by_key"
)]
fn get_metadata_cache(opts: GetMetadataOpts) -> Result<ImageMetadata> {
    trace!("SkopeoDriver::get_metadata({opts:#?})");

    let image_str = opts.image.to_string();

    let progress = Logger::multi_progress().add(
        ProgressBar::new_spinner()
            .with_style(ProgressStyle::default_spinner())
            .with_message(format!("Inspecting metadata for {}", image_str.bold())),
    );
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut command = cmd!(
        "skopeo",
        if !matches!(opts.platform, Platform::Native) => [
            "--override-arch",
            opts.platform.arch(),
        ],
        "inspect",
        format!("docker://{image_str}"),
    );
    command.stderr(Stdio::inherit());
    trace!("{command:?}");

    let output = command.output().into_diagnostic()?;

    progress.finish_and_clear();
    Logger::multi_progress().remove(&progress);

    if output.status.success() {
        debug!("Successfully inspected image {}!", image_str.bold().green());
    } else {
        bail!("Failed to inspect image {}", image_str.bold().red());
    }
    serde_json::from_slice(&output.stdout).into_diagnostic()
}

impl super::OciCopy for SkopeoDriver {
    fn copy_oci_dir(opts: CopyOciDirOpts) -> Result<()> {
        use crate::logging::CommandLogging;

        let use_sudo = opts.privileged && !blue_build_utils::running_as_root();
        let status = {
            let c = cmd!(
                if use_sudo {
                    "sudo"
                } else {
                    "skopeo"
                },
                if use_sudo && blue_build_utils::has_env_var(blue_build_utils::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    format!(
                        concat!(
                            "Password is required to copy ",
                            "OCI directory {dir:?} to remote registry {registry}"
                        ),
                        dir = opts.oci_dir,
                        registry = opts.registry,
                    )
                ],
                if use_sudo => "skopeo",
                "copy",
                opts.oci_dir,
                format!("docker://{}", opts.registry),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.registry.to_string(),
            format!("Copying {} to", opts.oci_dir),
        )
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to copy {} to {}", opts.oci_dir, opts.registry);
        }

        Ok(())
    }
}
