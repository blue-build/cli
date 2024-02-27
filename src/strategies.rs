use std::{env, path::PathBuf, rc::Rc};

use anyhow::{bail, Result};
use blue_build_utils::constants::*;
use log::trace;
use uuid::Uuid;

#[cfg(feature = "podman-api")]
use podman_api::Podman;

#[cfg(feature = "tokio")]
use tokio::runtime::Runtime;

use crate::{
    commands::build::Credentials,
    strategies::{buildah_strategy::BuildahStrategy, podman_strategy::PodmanStrategy},
};

#[cfg(feature = "builtin-podman")]
use crate::strategies::podman_api_strategy::PodmanApiStrategy;

mod buildah_strategy;
#[cfg(feature = "builtin-podman")]
mod podman_api_strategy;
mod podman_strategy;

pub trait BuildStrategy {
    fn build(&self, image: &str) -> Result<()>;

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    fn push(&self, image: &str) -> Result<()>;

    fn login(&self) -> Result<()>;
}

pub fn determine_build_strategy(
    uuid: Uuid,
    creds: Option<Credentials>,
) -> Result<Rc<dyn BuildStrategy>> {
    trace!("BuildStrategy::determine_strategy({uuid})");

    Ok(
        match (
            env::var(XDG_RUNTIME_DIR),
            PathBuf::from(RUN_PODMAN_SOCK),
            PathBuf::from(VAR_RUN_PODMAN_PODMAN_SOCK),
            PathBuf::from(VAR_RUN_PODMAN_SOCK),
            blue_build_utils::check_command_exists("podman"),
            blue_build_utils::check_command_exists("buildah"),
        ) {
            #[cfg(feature = "builtin-podman")]
            (Ok(xdg_runtime), _, _, _, _, _)
                if PathBuf::from(format!("{xdg_runtime}/podman/podman.sock")).exists() =>
            {
                Rc::new(
                    PodmanApiStrategy::builder()
                        .client(
                            Podman::unix(PathBuf::from(format!(
                                "{xdg_runtime}/podman/podman.sock"
                            )))
                            .into(),
                        )
                        .rt(Runtime::new()?)
                        .uuid(uuid)
                        .creds(creds)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, run_podman_podman_sock, _, _, _, _) if run_podman_podman_sock.exists() => Rc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(run_podman_podman_sock).into())
                    .rt(Runtime::new()?)
                    .uuid(uuid)
                    .creds(creds)
                    .build(),
            ),
            #[cfg(feature = "builtin-podman")]
            (_, _, var_run_podman_podman_sock, _, _, _) if var_run_podman_podman_sock.exists() => {
                Rc::new(
                    PodmanApiStrategy::builder()
                        .client(Podman::unix(var_run_podman_podman_sock).into())
                        .rt(Runtime::new()?)
                        .uuid(uuid)
                        .creds(creds)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, _, var_run_podman_sock, _, _) if var_run_podman_sock.exists() => Rc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .uuid(uuid)
                    .creds(creds)
                    .build(),
            ),
            (_, _, _, _, Ok(()), _) => Rc::new(PodmanStrategy::builder().creds(creds).build()),
            (_, _, _, _, _, Ok(())) => Rc::new(BuildahStrategy::builder().creds(creds).build()),
            _ => bail!("Could not determine strategy"),
        },
    )
}