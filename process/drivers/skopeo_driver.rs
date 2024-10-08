use std::{process::Stdio, time::Duration};

use blue_build_utils::cmd;
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
        trace!("SkopeoDriver::get_metadata({opts:#?})");

        let url = opts.tag.as_ref().map_or_else(
            || format!("docker://{}", opts.image),
            |tag| format!("docker://{}:{tag}", opts.image),
        );

        let progress = Logger::multi_progress().add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner())
                .with_message(format!("Inspecting metadata for {}", url.bold())),
        );
        progress.enable_steady_tick(Duration::from_millis(100));

        let mut command = cmd!(
            "skopeo",
            if !matches!(opts.platform, Platform::Native) => [
                "--override-arch",
                opts.platform.arch(),
            ],
            "inspect",
            &url,
            stderr = Stdio::inherit(),
        );
        trace!("{command:?}");

        let output = command.output().into_diagnostic()?;

        progress.finish_and_clear();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }
        serde_json::from_slice(&output.stdout).into_diagnostic()
    }
}
