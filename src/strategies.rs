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
pub use credentials::Credentials;
use log::{debug, error, info, trace};
use once_cell::sync::Lazy;
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
mod credentials;
mod docker_strategy;
#[cfg(feature = "builtin-podman")]
mod podman_api_strategy;
mod podman_strategy;
mod skopeo_strategy;

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
static BUILD_STRATEGY: Lazy<Arc<dyn BuildStrategy>> =
    Lazy::new(|| match Strategy::determine_build_strategy() {
        Err(e) => {
            error!("{e}");
            process::exit(1);
        }
        Ok(strat) => strat,
    });

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
static INSPECT_STRATEGY: Lazy<Arc<dyn InspectStrategy>> =
    Lazy::new(|| match Strategy::determine_inspect_strategy() {
        Err(e) => {
            error!("{e}");
            process::exit(1);
        }
        Ok(strat) => strat,
    });

/// UUID used to mark the current builds
static BUILD_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

/// The cached os versions
static OS_VERSION: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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

#[derive(Debug, TypedBuilder)]
pub struct Strategy<'a> {
    #[builder(default)]
    username: Option<&'a String>,

    #[builder(default)]
    password: Option<&'a String>,

    #[builder(default)]
    registry: Option<&'a String>,
}

impl<'a> Strategy<'a> {
    pub fn init(self) -> Result<()> {
        credentials::set_user_creds(self.username, self.password, self.registry)?;
        Ok(())
    }

    /// Gets the current build's UUID
    pub fn get_build_id() -> Uuid {
        *BUILD_ID
    }

    /// Gets the current run's build strategy
    pub fn get_build_strategy() -> Arc<dyn BuildStrategy> {
        BUILD_STRATEGY.clone()
    }

    /// Gets the current run's inspectioin strategy
    pub fn get_inspection_strategy() -> Arc<dyn InspectStrategy> {
        INSPECT_STRATEGY.clone()
    }

    pub fn get_credentials() -> Result<&'static Credentials> {
        credentials::get_credentials()
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
                info!("Retrieving OS version from {image}. This might take a bit");
                let inspection =
                    INSPECT_STRATEGY.get_labels(&recipe.base_image, &recipe.image_version)?;

                let os_version = inspection.get_version().ok_or_else(|| {
                    anyhow!(
                        "Unable to get the OS version from the labels. Please check with the image author about using '{IMAGE_VERSION_LABEL}' to report the os version."
                    )
                })?;
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

    fn determine_inspect_strategy() -> Result<Arc<dyn InspectStrategy>> {
        trace!("Strategy::determine_inspect_strategy()");

        Ok(
            match (
                blue_build_utils::check_command_exists("skopeo"),
                blue_build_utils::check_command_exists("docker"),
                blue_build_utils::check_command_exists("podman"),
            ) {
                (Ok(_skopeo), _, _) => Arc::new(SkopeoStrategy),
                (_, Ok(_docker), _) => Arc::new(DockerStrategy),
                (_, _, Ok(_podman)) => Arc::new(PodmanStrategy),
                _ => bail!("Could not determine inspection strategy. You need either skopeo, docker, or podman"),
            }
        )
    }

    fn determine_build_strategy() -> Result<Arc<dyn BuildStrategy>> {
        trace!("Strategy::determine_build_strategy()");

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
                (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => {
                    Arc::new(
                        PodmanApiStrategy::builder()
                            .client(Podman::unix(var_run_podman_sock).into())
                            .rt(Runtime::new()?)
                            .build(),
                    )
                }
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
}
