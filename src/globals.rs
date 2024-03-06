#![allow(clippy::redundant_closure_call)]
use std::{
    env,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
};

use anyhow::{bail, Result};
use blue_build_utils::constants::*;
use lazy_static::lazy_static;
use log::{error, trace};
use uuid::Uuid;

#[cfg(feature = "podman-api")]
use podman_api::Podman;

#[cfg(feature = "tokio")]
use tokio::runtime::Runtime;

use crate::strategies::{
    buildah_strategy::BuildahStrategy, docker_strategy::DockerStrategy,
    podman_strategy::PodmanStrategy, BuildStrategy,
};

#[cfg(feature = "builtin-podman")]
use crate::strategies::podman_api_strategy::PodmanApiStrategy;

use crate::strategies::Credentials;

pub struct UserCreds {
    username: Option<String>,
    password: Option<String>,
    registry: Option<String>,
}

lazy_static! {
    /// Stored global user creds.
    ///
    /// This is a special handoff static ref that is consumed
    /// by the `ENV_CREDENTIALS` static ref. This can be set
    /// at the beginning of a command for future calls for
    /// creds to source from.
    ///
    /// Order must be `username, password, registry`
    pub static ref USER_CREDS: Mutex<UserCreds> =
        Mutex::new(UserCreds { username: None, password: None, registry:  None });

    /// Stores the global env credentials.
    ///
    /// This on load will determine the credentials based off of
    /// `USER_CREDS` and env vars from CI systems. Once this is called
    /// the value is stored and cannot change.
    ///
    /// If you have user
    /// provided credentials, make sure you update `USER_CREDS`
    /// before trying to access this reference.
    pub static ref ENV_CREDENTIALS: Option<Credentials> = {
        let (username, password, registry) = {
            USER_CREDS.lock().map_or((None, None, None), |creds| (
                creds.username.clone(),
                creds.password.clone(),
                creds.registry.clone(),
            ))
        };

        let registry = match (
            registry.as_ref(),
            env::var(CI_REGISTRY).ok(),
            env::var(GITHUB_ACTIONS).ok(),
        ) {
            (Some(registry), _, _) => registry.to_owned(),
            (None, Some(ci_registry), None) => ci_registry,
            (None, None, Some(_)) => "ghcr.io".to_string(),
            _ => return None,
        };

        let username = match (
            username.as_ref(),
            env::var(CI_REGISTRY_USER).ok(),
            env::var(GITHUB_ACTOR).ok(),
        ) {
            (Some(username), _, _) => username.to_owned(),
            (None, Some(ci_registry_user), None) => ci_registry_user,
            (None, None, Some(github_actor)) => github_actor,
            _ => return None,
        };

        let password = match (
            password.as_ref(),
            env::var(CI_REGISTRY_PASSWORD).ok(),
            env::var(GITHUB_TOKEN).ok(),
        ) {
            (Some(password), _, _) => password.to_owned(),
            (None, Some(ci_registry_password), None) => ci_registry_password,
            (None, None, Some(registry_token)) => registry_token,
            _ => return None,
        };

        Some(
            Credentials::builder()
                .registry(registry)
                .username(username)
                .password(password)
                .build(),
        )
    };

    /// Stores the build strategy.
    ///
    /// This will, on load, find the best way to build in the
    /// current environment. Once that strategy is determined,
    /// it will be available for any part of the program to call
    /// on to perform builds.
    ///
    /// # Exits
    ///
    /// This will cause the program to exit if a build strategy could
    /// not be determined.
    pub static ref BUILD_STRATEGY: Arc<dyn BuildStrategy> = {
        match determine_build_strategy() {
            Err(e) => {
                error!("{e}");
                process::exit(1);
            }
            Ok(strat) => strat,
        }
    };
}

fn determine_build_strategy() -> Result<Arc<dyn BuildStrategy>> {
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
                Arc::new(
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
                Arc::new(
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
                Arc::new(
                    PodmanApiStrategy::builder()
                        .client(Podman::unix(var_run_podman_podman_sock).into())
                        .rt(Runtime::new()?)
                        .uuid(build_id)
                        .creds(creds)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => Arc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .uuid(build_id)
                    .creds(creds)
                    .build(),
            ),
            // (_, _, _, _, Ok(_docker), _, _) if !oci_required => {
            (_, _, _, _, Ok(_docker), _, _) => {
                Arc::new(DockerStrategy::builder().creds(creds).build())
            }
            (_, _, _, _, _, Ok(_podman), _) => {
                Arc::new(PodmanStrategy::builder().creds(creds).build())
            }
            (_, _, _, _, _, _, Ok(_buildah)) => {
                Arc::new(BuildahStrategy::builder().creds(creds).build())
            }
            _ => bail!(
                "Could not determine strategy, need either docker, podman, or buildah to continue"
            ),
        },
    )
}
