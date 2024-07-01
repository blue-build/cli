//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    path::PathBuf,
    sync::Mutex,
};

use anyhow::{anyhow, bail, Result};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use log::{debug, info, trace};
use once_cell::sync::Lazy;
use semver::{Version, VersionReq};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::{credentials, drivers::types::DetermineDriver, image_metadata::ImageMetadata};

use self::{
    buildah_driver::BuildahDriver,
    cosign_driver::CosignDriver,
    docker_driver::DockerDriver,
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, TagOpts},
    podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
    types::{BuildDriverType, InspectDriverType, SigningDriverType},
};

mod buildah_driver;
mod cosign_driver;
mod docker_driver;
pub mod opts;
mod podman_driver;
mod skopeo_driver;
pub mod types;

static INIT: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static SELECTED_BUILD_DRIVER: Lazy<Mutex<Option<BuildDriverType>>> = Lazy::new(|| Mutex::new(None));
static SELECTED_INSPECT_DRIVER: Lazy<Mutex<Option<InspectDriverType>>> =
    Lazy::new(|| Mutex::new(None));
static SELECTED_SIGNING_DRIVER: Lazy<Mutex<Option<SigningDriverType>>> =
    Lazy::new(|| Mutex::new(None));

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
pub trait BuildDriver {
    /// Runs the build logic for the strategy.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn build(opts: &BuildOpts) -> Result<()>;

    /// Runs the tag logic for the strategy.
    ///
    /// # Errors
    /// Will error if the tagging fails.
    fn tag(opts: &TagOpts) -> Result<()>;

    /// Runs the push logic for the strategy
    ///
    /// # Errors
    /// Will error if the push fails.
    fn push(opts: &PushOpts) -> Result<()>;

    /// Runs the login logic for the strategy.
    ///
    /// # Errors
    /// Will error if login fails.
    fn login() -> Result<()>;

    /// Runs the logic for building, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, tagging, or pusing fails.
    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<()> {
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
        Self::build(&build_opts)?;

        if !opts.tags.is_empty() && opts.archive_path.is_none() {
            let image = opts
                .image
                .as_ref()
                .ok_or_else(|| anyhow!("Image is required in order to tag"))?;
            debug!("Tagging all images");

            for tag in opts.tags.as_ref() {
                debug!("Tagging {} with {tag}", &full_image);

                let tag_opts = TagOpts::builder()
                    .src_image(&full_image)
                    .dest_image(format!("{image}:{tag}"))
                    .build();

                Self::tag(&tag_opts)?;

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

                        Self::push(&push_opts)
                    })?;
                }
            }
        }

        Ok(())
    }
}

/// Allows agnostic inspection of images.
pub trait InspectDriver {
    /// Gets the metadata on an image tag.
    ///
    /// # Errors
    /// Will error if it is unable to get the labels.
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata>;
}

pub trait SigningDriver {
    /// Generate a new private/public key pair.
    ///
    /// # Errors
    /// Will error if a key-pair couldn't be generated.
    fn generate_key_pair() -> Result<(PathBuf, PathBuf)>;

    /// Checks the signing key files to ensure
    /// they match.
    ///
    /// # Errors
    /// Will error if the files cannot be verified.
    fn check_signing_files() -> Result<()>;

    /// Sign an image given the image name and tag.
    ///
    /// # Errors
    /// Will error if the image fails to be signed.
    fn sign_images<S, T>(image_name: S, tag: Option<T>) -> Result<()>
    where
        S: AsRef<str>,
        T: AsRef<str> + Debug;
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
    signing_driver: Option<SigningDriverType>,
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
    ///
    /// # Panics
    /// Will panic if mutexes couldn't be locked.
    pub fn init(mut self) -> Result<()> {
        trace!("Driver::init()");
        let init = INIT.lock().expect("Should lock");
        credentials::set_user_creds(self.username, self.password, self.registry)?;

        let mut build_driver = SELECTED_BUILD_DRIVER.lock().expect("Should lock");
        let mut inspect_driver = SELECTED_INSPECT_DRIVER.lock().expect("Should lock");
        let mut signing_driver = SELECTED_SIGNING_DRIVER.lock().expect("Should lock");

        *signing_driver = Some(self.signing_driver.determine_driver());
        trace!("Inspect driver set to {:?}", *signing_driver);
        drop(signing_driver);

        *inspect_driver = Some(self.inspect_driver.determine_driver());
        trace!("Inspect driver set to {:?}", *inspect_driver);
        drop(inspect_driver);

        *build_driver = Some(self.build_driver.determine_driver());
        trace!("Build driver set to {:?}", *build_driver);
        drop(build_driver);

        drop(init);

        Ok(())
    }

    /// Gets the current build's UUID
    #[must_use]
    pub fn get_build_id() -> Uuid {
        trace!("Driver::get_build_id()");
        *BUILD_ID
    }

    /// Retrieve the `os_version` for an image.
    ///
    /// This gets cached for faster resolution if it's required
    /// in another part of the program.
    ///
    /// # Errors
    /// Will error if the image doesn't have OS version info
    /// or we are unable to lock a mutex.
    pub fn get_os_version(recipe: &Recipe) -> Result<u64> {
        trace!("Driver::get_os_version({recipe:#?})");
        let image = format!("{}:{}", &recipe.base_image, &recipe.image_version);

        let mut os_version_lock = OS_VERSION
            .lock()
            .map_err(|e| anyhow!("Unable set OS_VERSION {e}"))?;

        let entry = os_version_lock.get(&image);

        let os_version = match entry {
            None => {
                info!("Retrieving OS version from {image}. This might take a bit");
                let inspect_opts = GetMetadataOpts::builder()
                    .image(recipe.base_image.as_ref())
                    .tag(recipe.image_version.as_ref())
                    .build();
                let inspection = Self::get_metadata(&inspect_opts)?;

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

    fn get_build_driver() -> BuildDriverType {
        let lock = SELECTED_BUILD_DRIVER.lock().expect("Should lock");
        lock.expect("Driver should have initialized build driver")
    }

    fn get_inspect_driver() -> InspectDriverType {
        let lock = SELECTED_INSPECT_DRIVER.lock().expect("Should lock");
        lock.expect("Driver should have initialized inspect driver")
    }

    fn get_signing_driver() -> SigningDriverType {
        let lock = SELECTED_SIGNING_DRIVER.lock().expect("Should lock");
        lock.expect("Driver should have initialized signing driver")
    }
}

impl BuildDriver for Driver<'_> {
    fn build(opts: &BuildOpts) -> Result<()> {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::build(opts),
            BuildDriverType::Podman => PodmanDriver::build(opts),
            BuildDriverType::Docker => DockerDriver::build(opts),
        }
    }

    fn tag(opts: &TagOpts) -> Result<()> {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::tag(opts),
            BuildDriverType::Podman => PodmanDriver::tag(opts),
            BuildDriverType::Docker => DockerDriver::tag(opts),
        }
    }

    fn push(opts: &PushOpts) -> Result<()> {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::push(opts),
            BuildDriverType::Podman => PodmanDriver::push(opts),
            BuildDriverType::Docker => DockerDriver::push(opts),
        }
    }

    fn login() -> Result<()> {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::login(),
            BuildDriverType::Podman => PodmanDriver::login(),
            BuildDriverType::Docker => DockerDriver::login(),
        }
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<()> {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::build_tag_push(opts),
            BuildDriverType::Podman => PodmanDriver::build_tag_push(opts),
            BuildDriverType::Docker => DockerDriver::build_tag_push(opts),
        }
    }
}

impl SigningDriver for Driver<'_> {
    fn generate_key_pair() -> Result<(PathBuf, PathBuf)> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::generate_key_pair(),
            SigningDriverType::Podman => todo!(),
            SigningDriverType::Docker => todo!(),
        }
    }

    fn check_signing_files() -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::check_signing_files(),
            SigningDriverType::Podman => todo!(),
            SigningDriverType::Docker => todo!(),
        }
    }

    fn sign_images<S, T>(image_name: S, tag: Option<T>) -> Result<()>
    where
        S: AsRef<str>,
        T: AsRef<str> + Debug,
    {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::sign_images(image_name, tag),
            SigningDriverType::Podman => todo!(),
            SigningDriverType::Docker => todo!(),
        }
    }
}

impl InspectDriver for Driver<'_> {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        match Self::get_inspect_driver() {
            InspectDriverType::Skopeo => SkopeoDriver::get_metadata(opts),
            InspectDriverType::Podman => PodmanDriver::get_metadata(opts),
            InspectDriverType::Docker => DockerDriver::get_metadata(opts),
        }
    }
}
