use anyhow::{anyhow, Result};
use log::{debug, trace};
use std::process::Command;

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
