//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    process::{ExitStatus, Output},
    sync::{Mutex, RwLock},
};

use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use bon::Builder;
use clap::Args;
use log::{debug, info, trace};
use miette::{miette, Result};
use oci_distribution::Reference;
use once_cell::sync::Lazy;
use opts::{GenerateImageNameOpts, GenerateTagsOpts};
#[cfg(feature = "sigstore")]
use sigstore_driver::SigstoreDriver;
use uuid::Uuid;

use self::{
    buildah_driver::BuildahDriver,
    cosign_driver::CosignDriver,
    docker_driver::DockerDriver,
    github_driver::GithubDriver,
    gitlab_driver::GitlabDriver,
    image_metadata::ImageMetadata,
    local_driver::LocalDriver,
    opts::{
        BuildOpts, BuildTagPushOpts, CheckKeyPairOpts, GenerateKeyPairOpts, GetMetadataOpts,
        PushOpts, RunOpts, SignOpts, TagOpts, VerifyOpts,
    },
    podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
    types::{
        BuildDriverType, CiDriverType, DetermineDriver, InspectDriverType, RunDriverType,
        SigningDriverType,
    },
};

pub use traits::*;

mod buildah_driver;
mod cosign_driver;
mod docker_driver;
mod functions;
mod github_driver;
mod gitlab_driver;
pub mod image_metadata;
mod local_driver;
pub mod opts;
mod podman_driver;
#[cfg(feature = "sigstore")]
mod sigstore_driver;
mod skopeo_driver;
mod traits;
pub mod types;

static INIT: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
static SELECTED_BUILD_DRIVER: Lazy<RwLock<Option<BuildDriverType>>> =
    Lazy::new(|| RwLock::new(None));
static SELECTED_INSPECT_DRIVER: Lazy<RwLock<Option<InspectDriverType>>> =
    Lazy::new(|| RwLock::new(None));
static SELECTED_RUN_DRIVER: Lazy<RwLock<Option<RunDriverType>>> = Lazy::new(|| RwLock::new(None));
static SELECTED_SIGNING_DRIVER: Lazy<RwLock<Option<SigningDriverType>>> =
    Lazy::new(|| RwLock::new(None));
static SELECTED_CI_DRIVER: Lazy<RwLock<Option<CiDriverType>>> = Lazy::new(|| RwLock::new(None));

/// UUID used to mark the current builds
static BUILD_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

/// The cached os versions
static OS_VERSION: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Args for selecting the various drivers to use for runtime.
///
/// If the args are left uninitialized, the program will determine
/// the best one available.
#[derive(Default, Clone, Copy, Debug, Builder, Args)]
pub struct DriverArgs {
    /// Select which driver to use to build
    /// your image.
    #[arg(short = 'B', long)]
    build_driver: Option<BuildDriverType>,

    /// Select which driver to use to inspect
    /// images.
    #[arg(short = 'I', long)]
    inspect_driver: Option<InspectDriverType>,

    /// Select which driver to use to sign
    /// images.
    #[arg(short = 'S', long)]
    signing_driver: Option<SigningDriverType>,

    /// Select which driver to use to run
    /// containers.
    #[arg(short = 'R', long)]
    run_driver: Option<RunDriverType>,
}

macro_rules! impl_driver_type {
    ($cache:ident) => {{
        let lock = $cache.read().expect("Should read");
        lock.expect("Driver should have initialized build driver")
    }};
}

macro_rules! impl_driver_init {
    (@) => { };
    ($init:ident; $($tail:tt)*) => {
        {
            let mut initialized = $init.lock().expect("Must lock INIT");

            if !*initialized {
                impl_driver_init!(@ $($tail)*);

                *initialized = true;
            }
        }
    };
    (@ default => $cache:ident; $($tail:tt)*) => {
        {
            let mut driver = $cache.write().expect("Should lock");

            impl_driver_init!(@ $($tail)*);

            *driver = Some(driver.determine_driver());
            ::log::trace!("Driver set {driver:?}");
            drop(driver);
        }
    };
    (@ $driver:expr => $cache:ident; $($tail:tt)*) => {
        {
            let mut driver = $cache.write().expect("Should lock");

            impl_driver_init!(@ $($tail)*);

            *driver = Some($driver.determine_driver());
            ::log::trace!("Driver set {driver:?}");
            drop(driver);
        }
    };
}

pub struct Driver;

impl Driver {
    /// Initializes the Strategy with user provided credentials.
    ///
    /// If you want to take advantage of a user's credentials,
    /// you will want to run init before trying to use any of
    /// the strategies.
    ///
    /// # Panics
    /// Will panic if it is unable to initialize drivers.
    pub fn init(mut args: DriverArgs) {
        trace!("Driver::init()");

        impl_driver_init! {
            INIT;
            args.build_driver => SELECTED_BUILD_DRIVER;
            args.inspect_driver => SELECTED_INSPECT_DRIVER;
            args.run_driver => SELECTED_RUN_DRIVER;
            args.signing_driver => SELECTED_SIGNING_DRIVER;
            default => SELECTED_CI_DRIVER;
        }
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
    ///
    /// # Panics
    /// Panics if the mutex fails to lock.
    pub fn get_os_version(oci_ref: &Reference) -> Result<u64> {
        #[cfg(test)]
        {
            let _ = oci_ref; // silence lint

            if true {
                return Ok(40);
            }
        }

        trace!("Driver::get_os_version({oci_ref:#?})");
        let mut os_version_lock = OS_VERSION.lock().expect("Should lock");

        let entry = os_version_lock.get(&oci_ref.to_string());

        let os_version = match entry {
            None => {
                info!("Retrieving OS version from {oci_ref}. This might take a bit");
                let inspect_opts = GetMetadataOpts::builder()
                    .image(format!(
                        "{}/{}",
                        oci_ref.resolve_registry(),
                        oci_ref.repository()
                    ))
                    .tag(oci_ref.tag().unwrap_or("latest"))
                    .build();
                let inspection = Self::get_metadata(&inspect_opts)?;

                let os_version = inspection.get_version().ok_or_else(|| {
                miette!(
                    help = format!("Please check with the image author about using '{IMAGE_VERSION_LABEL}' to report the os version."),
                    "Unable to get the OS version from the labels"
                )
            })?;
                trace!("os_version: {os_version}");

                os_version
            }
            Some(os_version) => {
                debug!("Found cached {os_version} for {oci_ref}");
                *os_version
            }
        };

        if let Entry::Vacant(entry) = os_version_lock.entry(oci_ref.to_string()) {
            trace!("Caching version {os_version} for {oci_ref}");
            entry.insert(os_version);
        }
        drop(os_version_lock);
        Ok(os_version)
    }

    fn get_build_driver() -> BuildDriverType {
        impl_driver_type!(SELECTED_BUILD_DRIVER)
    }

    fn get_inspect_driver() -> InspectDriverType {
        impl_driver_type!(SELECTED_INSPECT_DRIVER)
    }

    fn get_signing_driver() -> SigningDriverType {
        impl_driver_type!(SELECTED_SIGNING_DRIVER)
    }

    fn get_run_driver() -> RunDriverType {
        impl_driver_type!(SELECTED_RUN_DRIVER)
    }

    fn get_ci_driver() -> CiDriverType {
        impl_driver_type!(SELECTED_CI_DRIVER)
    }
}

macro_rules! impl_build_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_build_driver() {
            BuildDriverType::Buildah => BuildahDriver::$func($($args,)*),
            BuildDriverType::Podman => PodmanDriver::$func($($args,)*),
            BuildDriverType::Docker => DockerDriver::$func($($args,)*),
        }
    };
}

impl BuildDriver for Driver {
    fn build(opts: &BuildOpts) -> Result<()> {
        impl_build_driver!(build(opts))
    }

    fn tag(opts: &TagOpts) -> Result<()> {
        impl_build_driver!(tag(opts))
    }

    fn push(opts: &PushOpts) -> Result<()> {
        impl_build_driver!(push(opts))
    }

    fn login() -> Result<()> {
        impl_build_driver!(login())
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<Vec<String>> {
        impl_build_driver!(build_tag_push(opts))
    }
}

macro_rules! impl_signing_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::$func($($args,)*),

            #[cfg(feature = "sigstore")]
            SigningDriverType::Sigstore => SigstoreDriver::$func($($args,)*),
        }
    };
}

impl SigningDriver for Driver {
    fn generate_key_pair(opts: &GenerateKeyPairOpts) -> Result<()> {
        impl_signing_driver!(generate_key_pair(opts))
    }

    fn check_signing_files(opts: &CheckKeyPairOpts) -> Result<()> {
        impl_signing_driver!(check_signing_files(opts))
    }

    fn sign(opts: &SignOpts) -> Result<()> {
        impl_signing_driver!(sign(opts))
    }

    fn verify(opts: &VerifyOpts) -> Result<()> {
        impl_signing_driver!(verify(opts))
    }

    fn signing_login() -> Result<()> {
        impl_signing_driver!(signing_login())
    }
}

macro_rules! impl_inspect_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_inspect_driver() {
            InspectDriverType::Skopeo => SkopeoDriver::$func($($args,)*),
            InspectDriverType::Podman => PodmanDriver::$func($($args,)*),
            InspectDriverType::Docker => DockerDriver::$func($($args,)*),
        }
    };
}

impl InspectDriver for Driver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        impl_inspect_driver!(get_metadata(opts))
    }
}

macro_rules! impl_run_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_run_driver() {
            RunDriverType::Docker => DockerDriver::$func($($args,)*),
            RunDriverType::Podman => PodmanDriver::$func($($args,)*),
        }
    };
}

impl RunDriver for Driver {
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus> {
        impl_run_driver!(run(opts))
    }

    fn run_output(opts: &RunOpts) -> std::io::Result<Output> {
        impl_run_driver!(run_output(opts))
    }
}

macro_rules! impl_ci_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::$func($($args,)*),
            CiDriverType::Gitlab => GitlabDriver::$func($($args,)*),
            CiDriverType::Github => GithubDriver::$func($($args,)*),
        }
    };
}

impl CiDriver for Driver {
    fn on_default_branch() -> bool {
        impl_ci_driver!(on_default_branch())
    }

    fn keyless_cert_identity() -> Result<String> {
        impl_ci_driver!(keyless_cert_identity())
    }

    fn oidc_provider() -> Result<String> {
        impl_ci_driver!(oidc_provider())
    }

    fn generate_tags(opts: &GenerateTagsOpts) -> Result<Vec<String>> {
        impl_ci_driver!(generate_tags(opts))
    }

    fn get_repo_url() -> Result<String> {
        impl_ci_driver!(get_repo_url())
    }

    fn get_registry() -> Result<String> {
        impl_ci_driver!(get_registry())
    }

    fn generate_image_name<'a, O>(opts: O) -> Result<Reference>
    where
        O: Borrow<GenerateImageNameOpts<'a>>,
    {
        impl_ci_driver!(generate_image_name(opts))
    }
}
