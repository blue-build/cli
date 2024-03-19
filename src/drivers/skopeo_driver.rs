use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use log::{debug, trace};

use crate::image_metadata::ImageMetadata;

use super::InspectDriver;

#[derive(Debug)]
pub struct SkopeoDriver;

impl InspectDriver for SkopeoDriver {
    fn get_metadata(&self, image_name: &str, tag: &str) -> Result<ImageMetadata> {
        let url = format!("docker://{image_name}:{tag}");

        trace!("skopeo inspect {url}");
        let output = Command::new("skopeo")
            .arg("inspect")
            .arg(&url)
            .stderr(Stdio::inherit())
            .output()?;

        if output.status.success() {
            debug!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
