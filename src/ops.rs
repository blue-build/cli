use anyhow::{anyhow, Result};
use log::{debug, trace};
use std::{path::Path, process::Command};

pub const LOCAL_BUILD: &str = "/etc/blue-build";
pub const ARCHIVE_SUFFIX: &str = "tar.gz";

pub fn check_command_exists(command: &str) -> Result<()> {
    trace!("check_command_exists({command})");
    debug!("Checking if {command} exists");

    trace!("which {command}");
    if Command::new("which")
        .arg(command)
        .output()?
        .status
        .success()
    {
        debug!("Command {command} does exist");
        Ok(())
    } else {
        Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        ))
    }
}

pub fn generate_local_image_name(image_name: &str, directory: Option<&str>) -> String {
    if let Some(directory) = directory {
        format!(
            "oci-archive:{}/{image_name}.{ARCHIVE_SUFFIX}",
            directory.trim_end_matches('/')
        )
    } else {
        format!("oci-archive:{image_name}.{ARCHIVE_SUFFIX}")
    }
}
