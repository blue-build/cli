pub mod command_output;
pub mod constants;

use std::{io::Write, path::PathBuf, process::Command, thread, time::Duration};

use anyhow::{anyhow, Result};
use format_serde_error::SerdeError;
use log::{debug, trace};

pub use command_output::*;

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

pub fn append_to_file(file_path: &str, content: &str) -> Result<()> {
    trace!("append_to_file({file_path}, {content})");
    debug!("Appending {content} to {file_path}");

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    writeln!(file, "\n{content}")?;
    Ok(())
}

pub fn serde_yaml_err(contents: &str) -> impl Fn(serde_yaml::Error) -> SerdeError + '_ {
    |err: serde_yaml::Error| {
        let location = err.location();
        let location = location.as_ref();
        SerdeError::new(
            contents.to_string(),
            (
                err.into(),
                location.map_or(0, serde_yaml::Location::line).into(),
                location.map_or(0, serde_yaml::Location::column).into(),
            ),
        )
    }
}

pub fn retry<V, F>(mut attempts: u8, delay: u64, f: F) -> anyhow::Result<V>
where
    F: Fn() -> anyhow::Result<V>,
{
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if attempts == 1 => return Err(e),
            _ => {
                attempts -= 1;
                thread::sleep(Duration::from_secs(delay));
            }
        };
    }
}

#[must_use]
pub fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|base_dirs| base_dirs.home_dir().to_path_buf())
}
