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
    strategies::{
        buildah_strategy::BuildahStrategy, docker_strategy::DockerStrategy,
        podman_strategy::PodmanStrategy,
    },
};

#[cfg(feature = "builtin-podman")]
use crate::strategies::podman_api_strategy::PodmanApiStrategy;

mod buildah_strategy;
mod docker_strategy;
#[cfg(feature = "builtin-podman")]
mod podman_api_strategy;
mod podman_strategy;

pub trait BuildStrategy {
    fn build(&self, image: &str) -> Result<()>;

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    fn push(&self, image: &str) -> Result<()>;

    fn login(&self) -> Result<()>;

    fn inspect(&self, image_name: &str, tag: &str) -> Result<Vec<u8>>;
}

pub fn determine_build_strategy(
    uuid: Uuid,
    creds: Option<Credentials>,
    oci_required: bool,
) -> Result<Rc<dyn BuildStrategy>> {
    trace!("BuildStrategy::determine_strategy({uuid})");

    Ok(
        match (
            env::var(XDG_RUNTIME_DIR),
            PathBuf::from(RUN_PODMAN_SOCK),
            PathBuf::from(VAR_RUN_PODMAN_PODMAN_SOCK),
            PathBuf::from(VAR_RUN_PODMAN_SOCK),
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
            blue_build_utils::check_command_exists("buildah"),
        ) {
            #[cfg(feature = "builtin-podman")]
            (Ok(xdg_runtime), _, _, _, _, _, _)
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
            (_, run_podman_podman_sock, _, _, _, _, _) if run_podman_podman_sock.exists() => {
                Rc::new(
                    PodmanApiStrategy::builder()
                        .client(Podman::unix(run_podman_podman_sock).into())
                        .rt(Runtime::new()?)
                        .uuid(uuid)
                        .creds(creds)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, var_run_podman_podman_sock, _, _, _, _)
                if var_run_podman_podman_sock.exists() =>
            {
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
            (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => Rc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .uuid(uuid)
                    .creds(creds)
                    .build(),
            ),
            (_, _, _, _, Ok(_docker), _, _) if !oci_required => {
                Rc::new(DockerStrategy::builder().creds(creds).build())
            }
            (_, _, _, _, _, Ok(_podman), _) => {
                Rc::new(PodmanStrategy::builder().creds(creds).build())
            }
            (_, _, _, _, _, _, Ok(_buildah)) => {
                Rc::new(BuildahStrategy::builder().creds(creds).build())
            }
            _ => bail!("Could not determine strategy"),
        },
    )
}
