//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    collections::{hash_map::Entry, HashMap},
    process::{ExitStatus, Output},
    sync::{Arc, Mutex},
};

use blue_build_recipe::Recipe;
// use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use log::{debug, info, trace};
use miette::{bail, miette, Result};
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::{credentials, image_metadata::ImageMetadata};

use self::{
    buildah_driver::BuildahDriver,
    docker_driver::DockerDriver,
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
    podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
    types::{BuildDriverType, InspectDriverType, RunDriverType},
};

mod buildah_driver;
mod docker_driver;
pub mod opts;
mod podman_driver;
mod skopeo_driver;
pub mod types;

static INIT: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
static SELECTED_BUILD_DRIVER: Lazy<Mutex<Option<BuildDriverType>>> = Lazy::new(|| Mutex::new(None));
static SELECTED_INSPECT_DRIVER: Lazy<Mutex<Option<InspectDriverType>>> =
    Lazy::new(|| Mutex::new(None));
static SELECTED_RUN_DRIVER: Lazy<Mutex<Option<RunDriverType>>> = Lazy::new(|| Mutex::new(None));

/// Stores the build driver.
///
/// This will, on load, find the best way to build in the
/// current environment. Once that strategy is determined,
/// it will be available for any part of the program to call
/// on to perform builds.
///
/// # Panics
///
/// This will cause a panic if a build strategy could
/// not be determined.
static BUILD_DRIVER: Lazy<Arc<dyn BuildDriver>> = Lazy::new(|| {
    let driver = SELECTED_BUILD_DRIVER.lock().unwrap();
    driver.map_or_else(
        || panic!("Driver needs to be initialized"),
        |driver| -> Arc<dyn BuildDriver> {
            match driver {
                BuildDriverType::Buildah => Arc::new(BuildahDriver),
                BuildDriverType::Podman => Arc::new(PodmanDriver),
                BuildDriverType::Docker => Arc::new(DockerDriver),
            }
        },
    )
});

/// Stores the inspection driver.
///
/// This will, on load, find the best way to inspect images in the
/// current environment. Once that strategy is determined,
/// it will be available for any part of the program to call
/// on to perform inspections.
///
/// # Panics
///
/// This will cause a panic if a inspect strategy could
/// not be determined.
static INSPECT_DRIVER: Lazy<Arc<dyn InspectDriver>> = Lazy::new(|| {
    let driver = SELECTED_INSPECT_DRIVER.lock().unwrap();
    driver.map_or_else(
        || panic!("Driver needs to be initialized"),
        |driver| -> Arc<dyn InspectDriver> {
            match driver {
                InspectDriverType::Skopeo => Arc::new(SkopeoDriver),
                InspectDriverType::Podman => Arc::new(PodmanDriver),
                InspectDriverType::Docker => Arc::new(DockerDriver),
            }
        },
    )
});

/// Stores the run driver.
///
/// This will, on load, find the best way to run containers in the
/// current environment. Once that strategy is determined,
/// it will be available for any part of the program to call
/// on to perform inspections.
///
/// # Panics
///
/// This will cause a panic if a run strategy could
/// not be determined.
static RUN_DRIVER: Lazy<Arc<dyn RunDriver>> = Lazy::new(|| {
    let driver = SELECTED_RUN_DRIVER.lock().unwrap();
    driver.map_or_else(
        || panic!("Driver needs to be initialized"),
        |driver| -> Arc<dyn RunDriver> {
            match driver {
                RunDriverType::Podman => Arc::new(PodmanDriver),
                RunDriverType::Docker => Arc::new(DockerDriver),
            }
        },
    )
});

/// UUID used to mark the current builds
static BUILD_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

/// The cached os versions
static OS_VERSION: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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
    fn build(&self, opts: &BuildOpts) -> Result<()>;

    /// Runs the tag logic for the strategy.
    ///
    /// # Errors
    /// Will error if the tagging fails.
    fn tag(&self, opts: &TagOpts) -> Result<()>;

    /// Runs the push logic for the strategy
    ///
    /// # Errors
    /// Will error if the push fails.
    fn push(&self, opts: &PushOpts) -> Result<()>;

    /// Runs the login logic for the strategy.
    ///
    /// # Errors
    /// Will error if login fails.
    fn login(&self) -> Result<()>;

    /// Runs the logic for building, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, tagging, or pusing fails.
    fn build_tag_push(&self, opts: &BuildTagPushOpts) -> Result<()> {
        trace!("BuildDriver::build_tag_push({opts:#?})");

        let full_image = match (opts.archive_path.as_ref(), opts.image.as_ref()) {
            (Some(archive_path), None) => {
                format!("oci-archive:{archive_path}")
            }
            (None, Some(image)) => opts
                .tags
                .first()
                .map_or_else(|| image.to_string(), |tag| format!("{image}:{tag}")),
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        };

        let build_opts = BuildOpts::builder()
            .image(&full_image)
            .containerfile(opts.containerfile.as_ref())
            .squash(opts.squash)
            .build();

        info!("Building image {full_image}");
        self.build(&build_opts)?;

        if !opts.tags.is_empty() && opts.archive_path.is_none() {
            let image = opts
                .image
                .as_ref()
                .ok_or_else(|| miette!("Image is required in order to tag"))?;
            debug!("Tagging all images");

            for tag in opts.tags.as_ref() {
                debug!("Tagging {} with {tag}", &full_image);

                let tag_opts = TagOpts::builder()
                    .src_image(&full_image)
                    .dest_image(format!("{image}:{tag}"))
                    .build();

                self.tag(&tag_opts)?;

                if opts.push {
                    let retry_count = if opts.no_retry_push {
                        0
                    } else {
                        opts.retry_count
                    };

                    debug!("Pushing all images");
                    // Push images with retries (1s delay between retries)
                    blue_build_utils::retry(retry_count, 1000, || {
                        let tag_image = format!("{image}:{tag}");

                        debug!("Pushing image {tag_image}");

                        let push_opts = PushOpts::builder()
                            .image(&tag_image)
                            .compression_type(opts.compression)
                            .build();

                        self.push(&push_opts)
                    })?;
                }
            }
        }

        Ok(())
    }
}

pub trait RunDriver: Sync + Send {
    /// Run a container to perform an action.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run(&self, opts: &RunOpts) -> std::io::Result<ExitStatus>;

    /// Run a container to perform an action and capturing output.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run_output(&self, opts: &RunOpts) -> std::io::Result<Output>;
}

/// Allows agnostic inspection of images.
pub trait InspectDriver: Sync + Send {
    /// Gets the metadata on an image tag.
    ///
    /// # Errors
    /// Will error if it is unable to get the labels.
    fn get_metadata(&self, opts: &GetMetadataOpts) -> Result<ImageMetadata>;
}

#[derive(Debug, TypedBuilder)]
pub struct Driver<'a> {
    #[builder(default)]
    username: Option<&'a String>,

    #[builder(default)]
    password: Option<&'a String>,

    #[builder(default)]
    registry: Option<&'a String>,

    #[builder(default)]
    build_driver: Option<BuildDriverType>,

    #[builder(default)]
    inspect_driver: Option<InspectDriverType>,

    #[builder(default)]
    run_driver: Option<RunDriverType>,
}

impl Driver<'_> {
    /// Initializes the Strategy with user provided credentials.
    ///
    /// If you want to take advantage of a user's credentials,
    /// you will want to run init before trying to use any of
    /// the strategies.
    ///
    /// # Panics
    /// Will panic if it is unable to initialize drivers.
    pub fn init(self) {
        trace!("Driver::init()");
        let mut initialized = INIT.lock().expect("Must lock INIT");

        if !*initialized {
            credentials::set_user_creds(self.username, self.password, self.registry);

            let mut build_driver = SELECTED_BUILD_DRIVER.lock().expect("Must lock BuildDriver");
            let mut inspect_driver = SELECTED_INSPECT_DRIVER
                .lock()
                .expect("Must lock InspectDriver");
            let mut run_driver = SELECTED_RUN_DRIVER.lock().expect("Must lock RunDriver");

            *build_driver = Some(
                self.build_driver
                    .map_or_else(Self::determine_build_driver, |driver| driver),
            );
            trace!("Build driver set to {:?}", *build_driver);
            drop(build_driver);
            let _ = Self::get_build_driver();

            *inspect_driver = Some(
                self.inspect_driver
                    .map_or_else(Self::determine_inspect_driver, |driver| driver),
            );
            trace!("Inspect driver set to {:?}", *inspect_driver);
            drop(inspect_driver);
            let _ = Self::get_inspection_driver();

            *run_driver = Some(
                self.run_driver
                    .map_or_else(Self::determine_run_driver, |driver| driver),
            );
            drop(run_driver);
            let _ = Self::get_run_driver();

            *initialized = true;
        }
    }

    /// Gets the current build's UUID
    #[must_use]
    pub fn get_build_id() -> Uuid {
        trace!("Driver::get_build_id()");
        *BUILD_ID
    }

    /// Gets the current run's build strategy
    pub fn get_build_driver() -> Arc<dyn BuildDriver> {
        trace!("Driver::get_build_driver()");
        BUILD_DRIVER.clone()
    }

    /// Gets the current run's inspectioin strategy
    pub fn get_inspection_driver() -> Arc<dyn InspectDriver> {
        trace!("Driver::get_inspection_driver()");
        INSPECT_DRIVER.clone()
    }

    pub fn get_run_driver() -> Arc<dyn RunDriver> {
        trace!("Driver::get_run_driver()");
        RUN_DRIVER.clone()
    }

    /// Retrieve the `os_version` for an image.
    ///
    /// This gets cached for faster resolution if it's required
    /// in another part of the program.
    ///
    /// # Errors
    /// Will error if the image doesn't have OS version info
    /// or we are unable to lock a mutex.
    ///
    /// # Panics
    /// Panics if the mutex fails to lock.
    pub fn get_os_version(recipe: &Recipe) -> Result<u64> {
        trace!("Driver::get_os_version({recipe:#?})");
        let image = format!("{}:{}", &recipe.base_image, &recipe.image_version);

        let mut os_version_lock = OS_VERSION.lock().expect("Should lock");

        let entry = os_version_lock.get(&image);

        let os_version = match entry {
            None => {
                info!("Retrieving OS version from {image}. This might take a bit");
                let inspect_opts = GetMetadataOpts::builder()
                    .image(recipe.base_image.as_ref())
                    .tag(recipe.image_version.as_ref())
                    .build();
                let inspection = INSPECT_DRIVER.get_metadata(&inspect_opts)?;

                let os_version = inspection.get_version().unwrap_or(0);
                trace!("os_version: {os_version}");

                os_version
            }
            Some(os_version) => {
                debug!("Found cached {os_version} for {image}");
                *os_version
            }
        };

        if let Entry::Vacant(entry) = os_version_lock.entry(image.clone()) {
            trace!("Caching version {os_version} for {image}");
            entry.insert(os_version);
        }
        drop(os_version_lock);
        Ok(os_version)
    }

    fn determine_inspect_driver() -> InspectDriverType {
        trace!("Driver::determine_inspect_driver()");

        match (
            blue_build_utils::check_command_exists("skopeo"),
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
        ) {
            (Ok(_skopeo), _, _) => InspectDriverType::Skopeo,
            (_, Ok(_docker), _) => InspectDriverType::Docker,
            (_, _, Ok(_podman)) => InspectDriverType::Podman,
            _ => panic!("Could not determine inspection strategy. You need either skopeo, docker, or podman"),
        }
    }

    fn determine_build_driver() -> BuildDriverType {
        trace!("Driver::determine_build_driver()");

        match (
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
            blue_build_utils::check_command_exists("buildah"),
        ) {
            (Ok(_docker), _, _) if DockerDriver::is_supported_version() => {
                BuildDriverType::Docker
            }
            (_, Ok(_podman), _) if PodmanDriver::is_supported_version() => {
                BuildDriverType::Podman
            }
            (_, _, Ok(_buildah)) if BuildahDriver::is_supported_version() => {
                BuildDriverType::Buildah
            }
            _ => panic!(
                "Could not determine strategy, need either docker version {}, podman version {}, or buildah version {} to continue",
                DockerDriver::VERSION_REQ,
                PodmanDriver::VERSION_REQ,
                BuildahDriver::VERSION_REQ,
            ),
        }
    }

    fn determine_run_driver() -> RunDriverType {
        trace!("Driver::determine_run_driver()");

        match (
            blue_build_utils::check_command_exists("docker"),
            blue_build_utils::check_command_exists("podman"),
        ) {
            (Ok(_docker), _) if DockerDriver::is_supported_version() => RunDriverType::Docker,
            (_, Ok(_podman)) if PodmanDriver::is_supported_version() => RunDriverType::Podman,
            _ => panic!(
                "{}{}{}{}",
                "Could not determine strategy, ",
                format_args!("need either docker version {}, ", DockerDriver::VERSION_REQ),
                format_args!("podman version {}, ", PodmanDriver::VERSION_REQ),
                format_args!(
                    "or buildah version {} to continue",
                    BuildahDriver::VERSION_REQ
                ),
            ),
        }
    }
}
