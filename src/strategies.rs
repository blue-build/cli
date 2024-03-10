//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    collections::{hash_map::Entry, HashMap},
    env,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Result};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::*;
use lazy_static::lazy_static;
use log::{debug, error, trace, warn};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[cfg(feature = "podman-api")]
use podman_api::Podman;

#[cfg(feature = "tokio")]
use tokio::runtime::Runtime;

#[cfg(feature = "builtin-podman")]
use crate::strategies::podman_api_strategy::PodmanApiStrategy;

use crate::image_inspection::ImageInspection;

use self::{
    buildah_strategy::BuildahStrategy, docker_strategy::DockerStrategy,
    podman_strategy::PodmanStrategy, skopeo_strategy::SkopeoStrategy,
};

mod buildah_strategy;
mod docker_strategy;
#[cfg(feature = "builtin-podman")]
mod podman_api_strategy;
mod podman_strategy;
mod skopeo_strategy;

lazy_static! {
    /// Stores the global env credentials.
    ///
    /// This on load will determine the credentials based off of
    /// `USER_CREDS` and env vars from CI systems. Once this is called
    /// the value is stored and cannot change.
    ///
    /// If you have user
    /// provided credentials, make sure you update `USER_CREDS`
    /// before trying to access this reference.
    static ref ENV_CREDENTIALS: Option<Credentials> = {
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

    /// Stores the inspection strategy.
    ///
    /// This will, on load, find the best way to inspect images in the
    /// current environment. Once that strategy is determined,
    /// it will be available for any part of the program to call
    /// on to perform inspections.
    ///
    /// # Exits
    ///
    /// This will cause the program to exit if a build strategy could
    /// not be determined.
    pub static ref INSPECT_STRATEGY: Arc<dyn InspectStrategy> = {
        match determine_inspect_strategy() {
            Err(e) => {
                error!("{e}");
                process::exit(1);
            }
            Ok(strat) => strat,
        }
    };

    pub static ref BUILD_ID: Uuid = Uuid::new_v4();

    /// The cached os versions
    static ref OS_VERSION: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

/// Stored user creds.
///
/// This is a special handoff static ref that is consumed
/// by the `ENV_CREDENTIALS` static ref. This can be set
/// at the beginning of a command for future calls for
/// creds to source from.
static USER_CREDS: Mutex<UserCreds> = Mutex::new(UserCreds {
    username: None,
    password: None,
    registry: None,
});

#[derive(Debug, Default, Clone, TypedBuilder)]
pub struct Credentials {
    pub registry: String,
    pub username: String,
    pub password: String,
}

struct UserCreds {
    pub username: Option<String>,
    pub password: Option<String>,
    pub registry: Option<String>,
}

/// Allows agnostic building, tagging
/// pushing, and login.
pub trait BuildStrategy: Sync + Send {
    fn build(&self, image: &str) -> Result<()>;

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    fn push(&self, image: &str) -> Result<()>;

    fn login(&self) -> Result<()>;
}

/// Allows agnostic inspection of images.
pub trait InspectStrategy: Sync + Send {
    fn get_labels(&self, image_name: &str, tag: &str) -> Result<ImageInspection>;
}

/// Retrieve the `os_version` for an image.
///
/// This gets cached for faster resolution if it's required
/// in another part of the program.
pub fn get_os_version(recipe: &Recipe) -> Result<String> {
    trace!("get_os_version({recipe:#?})");
    let image = format!("{}:{}", &recipe.base_image, &recipe.image_version);

    let mut os_version_lock = OS_VERSION
        .lock()
        .map_err(|e| anyhow!("Unable set OS_VERSION {e}"))?;

    let entry = os_version_lock.get(&image);

    let os_version = match entry {
        None => {
            let inspection =
                INSPECT_STRATEGY.get_labels(&recipe.base_image, &recipe.image_version)?;

            let os_version = inspection.get_version().unwrap_or_else(|| {
                warn!("Version label does not exist on image, using version in recipe");
                recipe.image_version.to_string()
            });
            trace!("os_version: {os_version}");

            os_version
        }
        Some(os_version) => {
            debug!("Found cached {os_version} for {image}");
            os_version.clone()
        }
    };

    if let Entry::Vacant(entry) = os_version_lock.entry(image.clone()) {
        trace!("Caching version {os_version} for {image}");
        entry.insert(os_version.clone());
    }
    drop(os_version_lock);
    Ok(os_version)
}

/// Set the users credentials for
/// the current set of actions.
///
/// Be sure to call this before trying to use
/// any strategy that requires credentials as
/// the environment credentials are lazy allocated.
pub fn set_user_creds(
    username: Option<&String>,
    password: Option<&String>,
    registry: Option<&String>,
) -> Result<()> {
    let mut creds_lock = USER_CREDS
        .lock()
        .map_err(|e| anyhow!("Failed to set credentials: {e}"))?;
    creds_lock.username = username.map(|u| u.to_owned());
    creds_lock.password = password.map(|p| p.to_owned());
    creds_lock.registry = registry.map(|r| r.to_owned());
    drop(creds_lock);
    Ok(())
}

fn determine_inspect_strategy() -> Result<Arc<dyn InspectStrategy>> {
    Ok(
        match (
            blue_build_utils::check_command_exists("skopeo"),
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
            blue_build_utils::check_command_exists("buildah"),
        ) {
            (Ok(_skopeo), _, _, _) => Arc::new(SkopeoStrategy),
            (_, Ok(_docker), _, _) => Arc::new(DockerStrategy),
            (_, _, Ok(_podman), _) => Arc::new(PodmanStrategy),
            (_, _, _, Ok(_buildah)) => Arc::new(BuildahStrategy),
            _ => bail!("Could not determine inspection strategy. You need eiterh skopeo, docker, podman, or buildah"),
        }
    )
}

fn determine_build_strategy() -> Result<Arc<dyn BuildStrategy>> {
    trace!("BuildStrategy::determine_strategy()");

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
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, run_podman_podman_sock, _, _, _, _, _) if run_podman_podman_sock.exists() => {
                Arc::new(
                    PodmanApiStrategy::builder()
                        .client(Podman::unix(run_podman_podman_sock).into())
                        .rt(Runtime::new()?)
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
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => Arc::new(
                PodmanApiStrategy::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .build(),
            ),
            // (_, _, _, _, Ok(_docker), _, _) if !oci_required => {
            (_, _, _, _, Ok(_docker), _, _) => Arc::new(DockerStrategy),
            (_, _, _, _, _, Ok(_podman), _) => Arc::new(PodmanStrategy),
            (_, _, _, _, _, _, Ok(_buildah)) => Arc::new(BuildahStrategy),
            _ => bail!(
                "Could not determine strategy, need either docker, podman, or buildah to continue"
            ),
        },
    )
}
