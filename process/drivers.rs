//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    process::{ExitStatus, Output},
    sync::{Mutex, RwLock},
};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use clap::Args;
use log::{debug, info, trace};
use miette::{miette, Result};
use once_cell::sync::Lazy;
use sigstore_driver::SigstoreDriver;
use typed_builder::TypedBuilder;
use users::{Groups, Users, UsersCache};
use uuid::Uuid;

use self::{
    buildah_driver::BuildahDriver,
    cosign_driver::CosignDriver,
    docker_driver::DockerDriver,
    github_driver::GithubDriver,
    gitlab_driver::GitlabDriver,
    image_metadata::ImageMetadata,
    local_driver::LocalDriver,
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
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
mod github_driver;
mod gitlab_driver;
pub mod image_metadata;
mod local_driver;
pub mod opts;
mod podman_driver;
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

static USER: Lazy<u32> = Lazy::new(|| UsersCache::new().get_current_uid());
static GROUP: Lazy<u32> = Lazy::new(|| UsersCache::new().get_current_gid());

/// Args for selecting the various drivers to use for runtime.
///
/// If the args are left uninitialized, the program will determine
/// the best one available.
#[derive(Default, Clone, Copy, Debug, TypedBuilder, Args)]
pub struct DriverArgs {
    /// Select which driver to use to build
    /// your image.
    #[builder(default)]
    #[arg(short = 'B', long)]
    build_driver: Option<BuildDriverType>,

    /// Select which driver to use to inspect
    /// images.
    #[builder(default)]
    #[arg(short = 'I', long)]
    inspect_driver: Option<InspectDriverType>,

    /// Select which driver to use to sign
    /// images.
    #[builder(default)]
    #[arg(short = 'S', long)]
    signing_driver: Option<SigningDriverType>,

    /// Select which driver to use to run
    /// containers.
    #[builder(default)]
    #[arg(short = 'R', long)]
    run_driver: Option<RunDriverType>,
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
        let mut initialized = INIT.lock().expect("Must lock INIT");

        if !*initialized {
            let mut build_driver = SELECTED_BUILD_DRIVER.write().expect("Should lock");
            let mut inspect_driver = SELECTED_INSPECT_DRIVER.write().expect("Should lock");
            let mut run_driver = SELECTED_RUN_DRIVER.write().expect("Should lock");
            let mut signing_driver = SELECTED_SIGNING_DRIVER.write().expect("Should lock");
            let mut ci_driver = SELECTED_CI_DRIVER.write().expect("Should lock");

            *ci_driver = Some(ci_driver.determine_driver());
            trace!("CI driver set to {:?}", *ci_driver);
            drop(ci_driver);

            *signing_driver = Some(args.signing_driver.determine_driver());
            trace!("Inspect driver set to {:?}", *signing_driver);
            drop(signing_driver);

            *run_driver = Some(args.run_driver.determine_driver());
            trace!("Run driver set to {:?}", *run_driver);
            drop(run_driver);

            *inspect_driver = Some(args.inspect_driver.determine_driver());
            trace!("Inspect driver set to {:?}", *inspect_driver);
            drop(inspect_driver);

            *build_driver = Some(args.build_driver.determine_driver());
            trace!("Build driver set to {:?}", *build_driver);
            drop(build_driver);

            *initialized = true;
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
    pub fn get_os_version(recipe: &Recipe) -> Result<u64> {
        #[cfg(test)]
        {
            use miette::IntoDiagnostic;

            if std::env::var(crate::test::BB_UNIT_TEST_MOCK_GET_OS_VERSION).is_ok() {
                return crate::test::create_test_recipe()
                    .image_version
                    .parse()
                    .into_diagnostic();
            }
        }

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
        let lock = SELECTED_BUILD_DRIVER.read().expect("Should read");
        lock.expect("Driver should have initialized build driver")
    }

    fn get_inspect_driver() -> InspectDriverType {
        let lock = SELECTED_INSPECT_DRIVER.read().expect("Should read");
        lock.expect("Driver should have initialized inspect driver")
    }

    fn get_signing_driver() -> SigningDriverType {
        let lock = SELECTED_SIGNING_DRIVER.read().expect("Should read");
        lock.expect("Driver should have initialized signing driver")
    }

    fn get_run_driver() -> RunDriverType {
        let lock = SELECTED_RUN_DRIVER.read().expect("Should read");
        lock.expect("Driver should have initialized run driver")
    }

    fn get_ci_driver() -> CiDriverType {
        let lock = SELECTED_CI_DRIVER.read().expect("Should read");
        lock.expect("Driver should have initialized CI driver")
    }
}

impl BuildDriver for Driver {
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

impl SigningDriver for Driver {
    fn generate_key_pair() -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::generate_key_pair(),
            SigningDriverType::Sigstore => SigstoreDriver::generate_key_pair(),
        }
    }

    fn check_signing_files() -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::check_signing_files(),
            SigningDriverType::Sigstore => SigstoreDriver::check_signing_files(),
        }
    }

    fn sign(image_digest: &str, key_arg: Option<String>) -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::sign(image_digest, key_arg),
            SigningDriverType::Sigstore => SigstoreDriver::sign(image_digest, key_arg),
        }
    }

    fn verify(image_name_tag: &str, verify_type: VerifyType) -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::verify(image_name_tag, verify_type),
            SigningDriverType::Sigstore => SigstoreDriver::verify(image_name_tag, verify_type),
        }
    }

    fn signing_login() -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::signing_login(),
            SigningDriverType::Sigstore => SigstoreDriver::signing_login(),
        }
    }
}

impl InspectDriver for Driver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        match Self::get_inspect_driver() {
            InspectDriverType::Skopeo => SkopeoDriver::get_metadata(opts),
            InspectDriverType::Podman => PodmanDriver::get_metadata(opts),
            InspectDriverType::Docker => DockerDriver::get_metadata(opts),
        }
    }
}

impl RunDriver for Driver {
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus> {
        match Self::get_run_driver() {
            RunDriverType::Podman => PodmanDriver::run(opts),
            RunDriverType::Docker => DockerDriver::run(opts),
        }
    }

    fn run_output(opts: &RunOpts) -> std::io::Result<Output> {
        match Self::get_run_driver() {
            RunDriverType::Podman => PodmanDriver::run_output(opts),
            RunDriverType::Docker => DockerDriver::run_output(opts),
        }
    }
}

impl CiDriver for Driver {
    fn on_default_branch() -> bool {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::on_default_branch(),
            CiDriverType::Gitlab => GitlabDriver::on_default_branch(),
            CiDriverType::Github => GithubDriver::on_default_branch(),
        }
    }

    fn keyless_cert_identity() -> Result<String> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::keyless_cert_identity(),
            CiDriverType::Gitlab => GitlabDriver::keyless_cert_identity(),
            CiDriverType::Github => GithubDriver::keyless_cert_identity(),
        }
    }

    fn oidc_provider() -> Result<String> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::oidc_provider(),
            CiDriverType::Gitlab => GitlabDriver::oidc_provider(),
            CiDriverType::Github => GithubDriver::oidc_provider(),
        }
    }

    fn generate_tags(recipe: &Recipe) -> Result<Vec<String>> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::generate_tags(recipe),
            CiDriverType::Gitlab => GitlabDriver::generate_tags(recipe),
            CiDriverType::Github => GithubDriver::generate_tags(recipe),
        }
    }

    fn get_repo_url() -> Result<String> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::get_repo_url(),
            CiDriverType::Gitlab => GitlabDriver::get_repo_url(),
            CiDriverType::Github => GithubDriver::get_repo_url(),
        }
    }

    fn get_registry() -> Result<String> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::get_registry(),
            CiDriverType::Gitlab => GitlabDriver::get_registry(),
            CiDriverType::Github => GithubDriver::get_registry(),
        }
    }

    fn generate_image_name(recipe: &Recipe) -> Result<String> {
        match Self::get_ci_driver() {
            CiDriverType::Local => LocalDriver::generate_image_name(recipe),
            CiDriverType::Gitlab => GitlabDriver::generate_image_name(recipe),
            CiDriverType::Github => GithubDriver::generate_image_name(recipe),
        }
    }
}
