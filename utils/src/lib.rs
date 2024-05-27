pub mod command_output;
pub mod constants;
pub mod logging;
pub mod syntax_highlighting;

use std::{ffi::OsStr, io::Write, path::PathBuf, process::Command, thread, time::Duration};

use anyhow::{anyhow, Result};
use format_serde_error::SerdeError;
use log::trace;

pub use command_output::*;

/// Checks for the existance of a given command.
///
/// # Errors
/// Will error if the command doesn't exist.
pub fn check_command_exists(command: &str) -> Result<()> {
    trace!("check_command_exists({command})");

    trace!("which {command}");
    if Command::new("which")
        .arg(command)
        .output()?
        .status
        .success()
    {
        trace!("Command {command} does exist");
        Ok(())
    } else {
        Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        ))
    }
}

/// Appends a string to a file.
///
/// # Errors
/// Will error if it fails to append to a file.
pub fn append_to_file<T: Into<PathBuf> + AsRef<OsStr>>(file_path: &T, content: &str) -> Result<()> {
    let file_path: PathBuf = file_path.into();
    trace!("append_to_file({}, {content})", file_path.display());

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    writeln!(file, "\n{content}")?;
    Ok(())
}

/// Creates a serde error for displaying the file
/// and where the error occurred.
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

/// Performs a retry on a given closure with a given nubmer of attempts and delay.
///
/// # Errors
/// Will error when retries have been expended.
pub fn retry<V, F>(attempts: u8, delay: u64, f: F) -> anyhow::Result<V>
where
    F: Fn() -> anyhow::Result<V>,
{
    let mut attempts = attempts;
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
