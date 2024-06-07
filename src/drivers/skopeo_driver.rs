use std::{
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Result};
use blue_build_utils::logging::Logger;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, trace};

use crate::image_metadata::ImageMetadata;

use super::{opts::GetMetadataOpts, InspectDriver};

#[derive(Debug)]
pub struct SkopeoDriver;

impl InspectDriver for SkopeoDriver {
    fn get_metadata(&self, opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("SkopeoDriver::get_metadata({opts:#?})");

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

        trace!("skopeo inspect {url}");
        let output = Command::new("skopeo")
            .arg("inspect")
            .arg(&url)
            .stderr(Stdio::inherit())
            .output()?;

        progress.finish();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
