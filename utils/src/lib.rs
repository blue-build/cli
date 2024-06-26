pub mod command_output;
pub mod constants;
pub mod logging;
pub mod signal_handler;
pub mod syntax_highlighting;

use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use base64::prelude::*;
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use format_serde_error::SerdeError;
use log::trace;

use crate::constants::CONTAINER_FILE;

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

/// Generates a 1-1 related Containerfile to a recipe.
/// The file is in the format of `Containerfile.{path_hash}`.
///
/// # Errors
/// Will error if unable to create a hash of the
pub fn generate_containerfile_path<T: AsRef<Path>>(path: T) -> Result<PathBuf> {
    const HASH_SIZE: usize = 8;
    let mut buf = [0u8; HASH_SIZE];

    let mut hasher = Blake2bVar::new(HASH_SIZE)?;
    hasher.update(path.as_ref().as_os_str().as_bytes());
    hasher.finalize_variable(&mut buf)?;

    Ok(PathBuf::from(format!(
        "{CONTAINER_FILE}.{}",
        BASE64_URL_SAFE_NO_PAD.encode(buf)
    )))
}
