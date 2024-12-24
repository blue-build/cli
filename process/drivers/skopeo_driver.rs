use std::{process::Stdio, time::Duration};

use blue_build_utils::cmd;
use cached::proc_macro::cached;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, trace};
use miette::{bail, IntoDiagnostic, Result};

use crate::{drivers::types::Platform, logging::Logger};

use super::{opts::GetMetadataOpts, types::ImageMetadata, InspectDriver};

#[derive(Debug)]
pub struct SkopeoDriver;

impl InspectDriver for SkopeoDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        get_metadata_cache(opts)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ format!("{}-{}", opts.image, opts.platform)}"#,
    sync_writes = true
)]
fn get_metadata_cache(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
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
        stderr = Stdio::inherit(),
    );
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

#[cfg(feature = "rechunk")]
impl super::OciCopy for SkopeoDriver {
    fn copy_oci_dir(
        oci_dir: &super::types::OciDir,
        registry: &oci_distribution::Reference,
    ) -> Result<()> {
        use crate::logging::CommandLogging;

        let status = {
            let c = cmd!("skopeo", "copy", oci_dir, format!("docker://{registry}"),);
            trace!("{c:?}");
            c
        }
        .build_status(registry.to_string(), format!("Copying {oci_dir} to"))
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to copy {oci_dir} to {registry}");
        }

        Ok(())
    }
}
