pub mod command_output;
pub mod constants;
pub mod credentials;
mod macros;
pub mod secret;
pub mod semver;
pub mod syntax_highlighting;
#[cfg(feature = "test")]
pub mod test_utils;
pub mod traits;

use std::{
    ops::{AsyncFnMut, Not},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use base64::prelude::*;
use blake2::{
    Blake2bVar,
    digest::{Update, VariableOutput},
};
use cached::proc_macro::once;
use chrono::{Local, Utc};
use comlexr::cmd;
use log::{trace, warn};
use miette::{Context, IntoDiagnostic, Result, miette};
use uuid::Uuid;

use crate::constants::CONTAINER_FILE;

pub use command_output::*;

/// UUID used to mark the current builds
pub static BUILD_ID: std::sync::LazyLock<Uuid> = std::sync::LazyLock::new(Uuid::new_v4);

/// Checks for the existance of a given command.
///
/// # Errors
/// Will error if the command doesn't exist.
pub fn check_command_exists(command: &str) -> Result<()> {
    trace!("check_command_exists({command})");

    trace!("which {command}");
    if cmd!("which", command)
        .output()
        .into_diagnostic()?
        .status
        .success()
    {
        trace!("Command {command} does exist");
        Ok(())
    } else {
        Err(miette!(
            "Command {command} doesn't exist and is required to build the image"
        ))
    }
}

/// Performs a retry on a given closure with a given nubmer of attempts and delay.
///
/// # Errors
/// Will error when retries have been expended.
pub fn retry<V, F>(mut retries: u8, delay_secs: u64, mut f: F) -> miette::Result<V>
where
    F: FnMut() -> miette::Result<V> + Send,
{
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if retries == 0 => return Err(e),
            Err(e) => {
                retries -= 1;
                warn!("Failed operation, will retry {retries} more time(s). Error:\n{e:?}");
                thread::sleep(Duration::from_secs(delay_secs));
            }
        }
    }
}

/// Performs a retry on a given closure with a given nubmer of attempts and delay.
///
/// # Errors
/// Will error when retries have been expended.
pub async fn retry_async<V, F>(mut retries: u8, delay_secs: u64, mut f: F) -> miette::Result<V>
where
    F: AsyncFnMut() -> miette::Result<V>,
{
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if retries == 0 => return Err(e),
            Err(e) => {
                retries -= 1;
                warn!("Failed operation, will retry {retries} more time(s). Error:\n{e:?}");
                thread::sleep(Duration::from_secs(delay_secs));
            }
        }
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

    let mut hasher = Blake2bVar::new(HASH_SIZE).into_diagnostic()?;
    hasher.update(path.as_ref().as_os_str().as_bytes());
    hasher.finalize_variable(&mut buf).into_diagnostic()?;

    Ok(PathBuf::from(format!(
        "{CONTAINER_FILE}.{}",
        BASE64_URL_SAFE_NO_PAD.encode(buf)
    )))
}

#[must_use]
pub fn get_tag_timestamp() -> String {
    Local::now().format("%Y%m%d").to_string()
}

/// Get's the env var wrapping it with a miette error
///
/// # Errors
/// Will error if the env var doesn't exist.
pub fn get_env_var(key: &str) -> Result<String> {
    std::env::var(key)
        .into_diagnostic()
        .with_context(|| format!("Failed to get {key}'"))
}

/// Checks if an environment variable is set and isn't empty.
#[must_use]
pub fn has_env_var(key: &str) -> bool {
    get_env_var(key).is_ok_and(|v| v.is_empty().not())
}

/// Checks if the process is running as root.
///
/// This call is cached to reduce syscalls.
#[once]
#[must_use]
pub fn running_as_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}

#[must_use]
pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
