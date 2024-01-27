use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use log::trace;

use crate::ops;

#[cfg(feature = "podman-api")]
#[derive(Debug, Clone, Default)]
pub enum BuildStrategy {
    #[default]
    Uninitialized,
    Socket(PathBuf),
    Buildah,
    Podman,
}

#[cfg(feature = "podman-api")]
impl BuildStrategy {
    pub fn determine_strategy() -> Result<Self> {
        trace!("BuildStrategy::determin_strategy()");

        Ok(
            match (
                env::var("XDG_RUNTIME_DIR"),
                PathBuf::from("/run/podman/podman.sock"),
                PathBuf::from("/var/run/podman/podman.sock"),
                PathBuf::from("/var/run/podman.sock"),
                ops::check_command_exists("buildah"),
                ops::check_command_exists("podman"),
            ) {
                (Ok(xdg_runtime), _, _, _, _, _)
                    if Path::new(&format!("{xdg_runtime}/podman/podman.sock")).exists() =>
                {
                    Self::Socket(PathBuf::from(format!("{xdg_runtime}/podman/podman.sock")))
                }
                (_, run_podman_podman_sock, _, _, _, _) if run_podman_podman_sock.exists() => {
                    Self::Socket(run_podman_podman_sock)
                }
                (_, _, var_run_podman_podman_sock, _, _, _)
                    if var_run_podman_podman_sock.exists() =>
                {
                    Self::Socket(var_run_podman_podman_sock)
                }
                (_, _, _, var_run_podman_sock, _, _) if var_run_podman_sock.exists() => {
                    Self::Socket(var_run_podman_sock)
                }
                (_, _, _, _, Ok(()), _) => Self::Buildah,
                (_, _, _, _, _, Ok(())) => Self::Podman,
                _ => bail!("Could not determine strategy"),
            },
        )
    }
}
