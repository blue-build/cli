use std::{env, path::PathBuf, rc::Rc, sync::Arc};

use anyhow::{bail, Result};
use blue_build_utils::constants::*;
use lazy_static::lazy_static;
use log::trace;
use once_cell::sync::Lazy;
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

static ENV_CREDENTIALS: Lazy<Option<Credentials>> = Lazy::new(|| {
    let registry = match (env::var(CI_REGISTRY).ok(), env::var(CI_REGISTRY).ok()) {
        (Some(ci_registry), None) => ci_registry,
        (None, Some(_)) => "ghcr.io".to_string(),
        _ => return None,
    };

    let username = match (env::var(CI_REGISTRY_USER).ok(), env::var(GITHUB_ACTOR).ok()) {
        (Some(ci_registry_user), None) => ci_registry_user,
        (None, Some(github_actor)) => github_actor,
        _ => return None,
    };

    let password = match (
        env::var(CI_REGISTRY_PASSWORD).ok(),
        env::var(GITHUB_TOKEN).ok(),
    ) {
        (Some(ci_registry_password), None) => ci_registry_password,
        (None, Some(registry_token)) => registry_token,
        _ => return None,
    };

    Some(
        Credentials::builder()
            .registry(registry)
            .username(username)
            .password(password)
            .build(),
    )
});

static ER: Lazy<Box<Rc<dyn BuildStrategy>>> =
    Lazy::new(|| Box::new(determine_build_strategy().unwrap()));

pub trait BuildStrategy: Sync + Send {
    fn build(&self, image: &str) -> Result<()>;

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    fn push(&self, image: &str) -> Result<()>;

    fn login(&self) -> Result<()>;

    fn inspect(&self, image_name: &str, tag: &str) -> Result<Vec<u8>>;
}

pub fn determine_build_strategy() -> Result<Rc<dyn BuildStrategy>> {
    let build_id = Uuid::new_v4();
    let creds = ENV_CREDENTIALS.to_owned();

    trace!("BuildStrategy::determine_strategy({build_id})");

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
                        .uuid(build_id)
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
                        .uuid(build_id)
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
                        .uuid(build_id)
                        .creds(creds)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => Rc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .uuid(build_id)
                    .creds(creds)
                    .build(),
            ),
            // (_, _, _, _, Ok(_docker), _, _) if !oci_required => {
            (_, _, _, _, Ok(_docker), _, _) => {
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
