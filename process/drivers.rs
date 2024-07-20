//! This module is responsible for managing various strategies
//! to perform actions throughout the program. This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    collections::{hash_map::Entry, HashMap},
    env,
    fmt::Debug,
    path::Path,
    process::{ExitStatus, Output},
    sync::{Mutex, RwLock},
};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::{
    COSIGN_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PUB_PATH, IMAGE_VERSION_LABEL,
};
use clap::Args;
use log::{debug, info, trace, warn};
use miette::{bail, miette, Result};
use once_cell::sync::Lazy;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use self::{
    buildah_driver::BuildahDriver,
    cosign_driver::{CosignDriver, VerifyType},
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
        T: AsRef<str>,
    {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::sign_images(image_name, tag),
            SigningDriverType::Podman => todo!(),
            SigningDriverType::Docker => todo!(),
        }
    }

    fn signing_login() -> Result<()> {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::signing_login(),
            SigningDriverType::Podman => todo!(),
            SigningDriverType::Docker => todo!(),
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

fn get_private_key(check_fn: impl FnOnce(String) -> Result<()>) -> Result<()> {
    match (
        Path::new(COSIGN_PUB_PATH).exists(),
        env::var(COSIGN_PRIVATE_KEY).ok(),
        Path::new(COSIGN_PRIV_PATH),
    ) {
        (true, Some(cosign_priv_key), _) if !cosign_priv_key.is_empty() => {
            check_fn("env://COSIGN_PRIVATE_KEY".to_string())
        }
        (true, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
            check_fn(cosign_priv_key_path.display().to_string())
        }
        (true, _, _) => {
            bail!(
                "{}{}{}{}{}{}{}",
                "Unable to find private/public key pair.\n\n",
                format_args!("Make sure you have a `{COSIGN_PUB_PATH}` "),
                format_args!("in the root of your repo and have either {COSIGN_PRIVATE_KEY} "),
                format_args!("set in your env variables or a `{COSIGN_PRIV_PATH}` "),
                "file in the root of your repo.\n\n",
                "See https://blue-build.org/how-to/cosign/ for more information.\n\n",
                "If you don't want to sign your image, use the `--no-sign` flag."
            )
        }
        _ => Ok(()),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn sign_images<S, T, Sign, Verify>(
    image_name: S,
    tag: Option<T>,
    sign_fn: Sign,
    verify_fn: Verify,
) -> Result<()>
where
    S: AsRef<str>,
    T: AsRef<str>,
    Sign: Fn(&str, Option<String>) -> Result<()>,
    Verify: Fn(&str, VerifyType) -> Result<()>,
{
    let image_name = image_name.as_ref();
    let tag = tag.as_ref().map(AsRef::as_ref);
    trace!("sign_images({image_name}, {tag:?}, sign_fn, verify_fn)");

    let inspect_opts = GetMetadataOpts::builder().image(image_name);

    let inspect_opts = if let Some(tag) = tag {
        inspect_opts.tag(tag).build()
    } else {
        inspect_opts.build()
    };

    let image_digest = Driver::get_metadata(&inspect_opts)?.digest;
    let image_name_tag = tag.map_or_else(|| image_name.to_owned(), |t| format!("{image_name}:{t}"));
    let image_digest = format!("{image_name}@{image_digest}");

    match (
        Driver::get_ci_driver(),
        // Cosign public/private key pair
        env::var(COSIGN_PRIVATE_KEY),
        Path::new(COSIGN_PRIV_PATH),
    ) {
        // Cosign public/private key pair
        (_, Ok(cosign_private_key), _)
            if !cosign_private_key.is_empty() && Path::new(COSIGN_PUB_PATH).exists() =>
        {
            sign_fn(
                &image_digest,
                Some(format!("--key=env://{COSIGN_PRIVATE_KEY}")),
            )?;
            verify_fn(&image_name_tag, VerifyType::File(COSIGN_PUB_PATH.into()))?;
        }
        (_, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
            sign_fn(
                &image_digest,
                Some(format!("--key={}", cosign_priv_key_path.display())),
            )?;
            verify_fn(&image_name_tag, VerifyType::File(COSIGN_PUB_PATH.into()))?;
        }
        // Gitlab keyless
        (CiDriverType::Github | CiDriverType::Gitlab, _, _) => {
            sign_fn(&image_digest, None)?;
            verify_fn(
                &image_name_tag,
                VerifyType::Keyless {
                    issuer: Driver::oidc_provider()?,
                    identity: Driver::keyless_cert_identity()?,
                },
            )?;
        }
        _ => warn!("Not running in CI with cosign variables, not signing"),
    }

    Ok(())
}
