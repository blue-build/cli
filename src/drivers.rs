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
use blue_build_utils::constants::{
    IMAGE_VERSION_LABEL, RUN_PODMAN_SOCK, VAR_RUN_PODMAN_PODMAN_SOCK, VAR_RUN_PODMAN_SOCK,
    XDG_RUNTIME_DIR,
};
use log::{debug, error, info, trace};
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[cfg(feature = "podman-api")]
use podman_api::Podman;

#[cfg(feature = "tokio")]
use tokio::runtime::Runtime;

#[cfg(feature = "builtin-podman")]
use podman_api_driver::PodmanApiDriver;

use crate::{credentials, image_inspection::ImageInspection};

use self::{
    buildah_driver::BuildahDriver, docker_driver::DockerDriver, podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
};

mod buildah_driver;
mod docker_driver;
#[cfg(feature = "builtin-podman")]
mod podman_api_driver;
mod podman_driver;
mod skopeo_driver;

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
static BUILD_STRATEGY: Lazy<Arc<dyn BuildDriver>> =
    Lazy::new(|| match Driver::determine_build_driver() {
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
static INSPECT_STRATEGY: Lazy<Arc<dyn InspectDriver>> =
    Lazy::new(|| match Driver::determine_inspect_driver() {
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

/// Trait for retrieving version of a driver.
pub trait DriverVersion {
    /// The version req string slice that follows
    /// the semver standard <https://semver.org/>.
    const VERSION_REQ: &'static str;

    /// Returns the version of the driver.
    ///
    /// # Errors
    /// Will error if it can't retrieve the version.
    fn version() -> Result<Version>;

    #[must_use]
    fn is_supported_version() -> bool {
        Self::version().is_ok_and(|version| {
            VersionReq::parse(Self::VERSION_REQ).is_ok_and(|req| req.matches(&version))
        })
    }
}

/// Allows agnostic building, tagging
/// pushing, and login.
pub trait BuildDriver: Sync + Send {
    /// Runs the build logic for the strategy.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn build(&self, image: &str) -> Result<()>;

    /// Runs the tag logic for the strategy.
    ///
    /// # Errors
    /// Will error if the tagging fails.
    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    /// Runs the push logic for the strategy
    ///
    /// # Errors
    /// Will error if the push fails.
    fn push(&self, image: &str) -> Result<()>;

    /// Runs the login logic for the strategy.
    ///
    /// # Errors
    /// Will error if login fails.
    fn login(&self) -> Result<()>;
}

/// Allows agnostic inspection of images.
pub trait InspectDriver: Sync + Send {
    /// Gets the labels on an image tag.
    ///
    /// # Errors
    /// Will error if it is unable to get the labels.
    fn get_labels(&self, image_name: &str, tag: &str) -> Result<ImageInspection>;
}

#[derive(Debug, TypedBuilder)]
pub struct Driver<'a> {
    #[builder(default)]
    username: Option<&'a String>,

    #[builder(default)]
    password: Option<&'a String>,

    #[builder(default)]
    registry: Option<&'a String>,
}

impl Driver<'_> {
    /// Initializes the Strategy with user provided credentials.
    ///
    /// If you want to take advantage of a user's credentials,
    /// you will want to run init before trying to use any of
    /// the strategies.
    ///
    /// # Errors
    /// Will error if it is unable to set the user credentials.
    pub fn init(self) -> Result<()> {
        credentials::set_user_creds(self.username, self.password, self.registry)?;
        Ok(())
    }

    /// Gets the current build's UUID
    #[must_use]
    pub fn get_build_id() -> Uuid {
        *BUILD_ID
    }

    /// Gets the current run's build strategy
    pub fn get_build_driver() -> Arc<dyn BuildDriver> {
        BUILD_STRATEGY.clone()
    }

    /// Gets the current run's inspectioin strategy
    pub fn get_inspection_driver() -> Arc<dyn InspectDriver> {
        INSPECT_STRATEGY.clone()
    }

    /// Retrieve the `os_version` for an image.
    ///
    /// This gets cached for faster resolution if it's required
    /// in another part of the program.
    ///
    /// # Errors
    /// Will error if the image doesn't have OS version info
    /// or we are unable to lock a mutex.
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

    fn determine_inspect_driver() -> Result<Arc<dyn InspectDriver>> {
        trace!("Strategy::determine_inspect_strategy()");

        let driver: Arc<dyn InspectDriver> = match (
            blue_build_utils::check_command_exists("skopeo"),
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
        ) {
            (Ok(_skopeo), _, _) => Arc::new(SkopeoDriver),
            (_, Ok(_docker), _) => Arc::new(DockerDriver),
            (_, _, Ok(_podman)) => Arc::new(PodmanDriver),
            _ => bail!("Could not determine inspection strategy. You need either skopeo, docker, or podman"),
        };

        Ok(driver)
    }

    fn determine_build_driver() -> Result<Arc<dyn BuildDriver>> {
        trace!("Strategy::determine_build_strategy()");

        let driver: Arc<dyn BuildDriver> = match (
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
                    PodmanApiDriver::builder()
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
                    PodmanApiDriver::builder()
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
                    PodmanApiDriver::builder()
                        .client(Podman::unix(var_run_podman_podman_sock).into())
                        .rt(Runtime::new()?)
                        .build(),
                )
            }
            #[cfg(feature = "builtin-podman")]
            (_, _, _, var_run_podman_sock, _, _, _) if var_run_podman_sock.exists() => Arc::new(
                PodmanApiDriver::builder()
                    .client(Podman::unix(var_run_podman_sock).into())
                    .rt(Runtime::new()?)
                    .build(),
            ),
            (_, _, _, _, Ok(_docker), _, _) if DockerDriver::is_supported_version() => {
                Arc::new(DockerDriver)
            }
            (_, _, _, _, _, Ok(_podman), _) if PodmanDriver::is_supported_version() => {
                Arc::new(PodmanDriver)
            }
            (_, _, _, _, _, _, Ok(_buildah)) if BuildahDriver::is_supported_version() => {
                Arc::new(BuildahDriver)
            }
            _ => bail!(
                "Could not determine strategy, need either docker version {}, podman version {}, or buildah version {} to continue",
                DockerDriver::VERSION_REQ,
                PodmanDriver::VERSION_REQ,
                BuildahDriver::VERSION_REQ,
            ),
        };

        Ok(driver)
    }
}
