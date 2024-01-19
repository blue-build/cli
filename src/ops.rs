use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Result};
use clap::ValueEnum;
use log::{debug, trace};

pub fn check_command_exists(command: &str) -> Result<()> {
    trace!("check_command_exists({command})");
    debug!("Checking if {command} exists");

    trace!("which {command}");
    match Command::new("which")
        .arg(command)
        .output()?
        .status
        .success()
    {
        true => {
            debug!("Command {command} does exist");
            Ok(())
        }
        false => Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        )),
    }
}
