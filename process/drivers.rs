//! This module is responsible for managing various strategies
//! to perform actions throughout the program.
//!
//! This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    borrow::Borrow,
    fmt::Debug,
    process::{ExitStatus, Output},
    sync::{LazyLock, RwLock, atomic::AtomicBool},
    time::Duration,
};

use blue_build_utils::{
    BUILD_ID,
    constants::{
        BB_BOOT_DRIVER, BB_BUILD_DRIVER, BB_INSPECT_DRIVER, BB_RUN_DRIVER, BB_SIGNING_DRIVER,
    },
    semver::Version,
};
use bon::{Builder, bon};
use cached::proc_macro::cached;
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, trace, warn};
use miette::{Context, Result};
use oci_distribution::Reference;
use opts::{
    BuildOpts, BuildTagPushOpts, CheckKeyPairOpts, ContainerOpts, CopyOciDirOpts,
    CreateContainerOpts, GenerateImageNameOpts, GenerateKeyPairOpts, GenerateTagsOpts,
    GetMetadataOpts, PruneOpts, PushOpts, RechunkOpts, RemoveContainerOpts, RemoveImageOpts,
    RunOpts, SignOpts, SwitchOpts, TagOpts, VerifyOpts, VolumeOpts,
};
use types::{
    BootDriverType, BuildDriverType, CiDriverType, ImageMetadata, InspectDriverType, RunDriverType,
    SigningDriverType,
};
use uuid::Uuid;

use crate::{drivers::oci_client::OciClientDriver, logging::Logger};

pub use self::{
    buildah_driver::BuildahDriver, cosign_driver::CosignDriver, docker_driver::DockerDriver,
    github_driver::GithubDriver, gitlab_driver::GitlabDriver, local_driver::LocalDriver,
    podman_driver::PodmanDriver, rpm_ostree_driver::RpmOstreeDriver,
    sigstore_driver::SigstoreDriver, skopeo_driver::SkopeoDriver, traits::*,
};

#[cfg(feature = "bootc")]
pub use bootc_driver::BootcDriver;

#[cfg(feature = "bootc")]
mod bootc_driver;
mod buildah_driver;
mod cosign_driver;
mod docker_driver;
mod functions;
mod github_driver;
mod gitlab_driver;
mod local_driver;
mod oci_client;
pub mod opts;
mod podman_driver;
mod rpm_ostree_driver;
mod sigstore_driver;
mod skopeo_driver;
mod traits;
pub mod types;

static INIT: AtomicBool = AtomicBool::new(false);
static SELECTED_BUILD_DRIVER: LazyLock<RwLock<Option<BuildDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_INSPECT_DRIVER: LazyLock<RwLock<Option<InspectDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_RUN_DRIVER: LazyLock<RwLock<Option<RunDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_SIGNING_DRIVER: LazyLock<RwLock<Option<SigningDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_CI_DRIVER: LazyLock<RwLock<Option<CiDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_BOOT_DRIVER: LazyLock<RwLock<Option<BootDriverType>>> =
    LazyLock::new(|| RwLock::new(None));

/// Args for selecting the various drivers to use for runtime.
///
/// If the args are left uninitialized, the program will determine
/// the best one available.
#[derive(Default, Clone, Copy, Debug, Builder, Args)]
#[allow(clippy::struct_field_names)]
pub struct DriverArgs {
    /// Select which driver to use to build
    /// your image.
    #[arg(short = 'B', long, env = BB_BUILD_DRIVER)]
    pub build_driver: Option<BuildDriverType>,

    /// Select which driver to use to inspect
    /// images.
    #[arg(short = 'I', long, env = BB_INSPECT_DRIVER)]
    pub inspect_driver: Option<InspectDriverType>,

    /// Select which driver to use to sign
    /// images.
    #[arg(short = 'S', long, env = BB_SIGNING_DRIVER)]
    pub signing_driver: Option<SigningDriverType>,

    /// Select which driver to use to run
    /// containers.
    #[arg(short = 'R', long, env = BB_RUN_DRIVER)]
    pub run_driver: Option<RunDriverType>,

    #[arg(short = 'T', long, env = BB_BOOT_DRIVER)]
    pub boot_driver: Option<BootDriverType>,
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
            if $init.compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire
            ).is_ok() {
                impl_driver_init!(@ $($tail)*);
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

#[bon]
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

        if args.inspect_driver.is_some() {
            warn!("Setting the inspect driver is deprecated.");
        }

        impl_driver_init! {
            INIT;
            args.build_driver => SELECTED_BUILD_DRIVER;
            args.run_driver => SELECTED_RUN_DRIVER;
            args.signing_driver => SELECTED_SIGNING_DRIVER;
            args.boot_driver => SELECTED_BOOT_DRIVER;
            default => SELECTED_INSPECT_DRIVER;
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
    #[builder]
    pub fn get_os_version(
        /// The OCI image reference.
        oci_ref: &Reference,
    ) -> Result<u64> {
        trace!("Driver::get_os_version({oci_ref:#?})");

        #[cfg(test)]
        {
            let _ = oci_ref; // silence lint

            if true {
                return Ok(41);
            }
        }

        info!("Retrieving OS version from {oci_ref}");

        let os_version = Self::get_metadata(GetMetadataOpts::builder().image(oci_ref).build())
            .and_then(|inspection| {
                trace!("{inspection:?}");
                inspection.get_version().wrap_err_with(|| {
                    format!(
                        "Failed to parse version from metadata for {}",
                        oci_ref.to_string().bold()
                    )
                })
            })
            .or_else(|err| {
                warn!("Unable to get version via image inspection due to error:\n{err:?}");
                get_version_run_image(oci_ref)
            })?;
        trace!("os_version: {os_version}");
        Ok(os_version.major)
    }

    pub fn get_build_driver() -> BuildDriverType {
        impl_driver_type!(SELECTED_BUILD_DRIVER)
    }

    pub fn get_inspect_driver() -> InspectDriverType {
        impl_driver_type!(SELECTED_INSPECT_DRIVER)
    }

    pub fn get_signing_driver() -> SigningDriverType {
        impl_driver_type!(SELECTED_SIGNING_DRIVER)
    }

    pub fn get_run_driver() -> RunDriverType {
        impl_driver_type!(SELECTED_RUN_DRIVER)
    }

    pub fn get_ci_driver() -> CiDriverType {
        impl_driver_type!(SELECTED_CI_DRIVER)
    }

    pub fn get_boot_driver() -> BootDriverType {
        impl_driver_type!(SELECTED_BOOT_DRIVER)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ oci_ref.to_string() }"#,
    sync_writes = "by_key"
)]
fn get_version_run_image(oci_ref: &Reference) -> Result<Version> {
    warn!(concat!(
        "Pulling and running the image to retrieve the version. ",
        "This will take a while..."
    ));

    let progress = Logger::multi_progress().add(
        ProgressBar::new_spinner()
            .with_style(ProgressStyle::default_spinner())
            .with_message(format!(
                "Pulling image {} to get version",
                oci_ref.to_string().bold()
            )),
    );
    progress.enable_steady_tick(Duration::from_millis(100));

    let should_remove = if matches!(Driver::get_run_driver(), RunDriverType::Docker) {
        !Driver::list_images(false)?.contains(oci_ref)
    } else {
        false
    };

    let output = Driver::run_output(
        RunOpts::builder()
            .image(&oci_ref.to_string())
            .args(&bon::vec![
                "/bin/bash",
                "-c",
                r#"awk -F= '/^VERSION_ID=/ {gsub(/"/, "", $2); print $2}' /usr/lib/os-release"#,
            ])
            .pull(true)
            .remove(true)
            .build(),
    )?;

    if should_remove {
        Driver::remove_image(RemoveImageOpts::builder().image(oci_ref).build())?;
    }

    progress.finish_and_clear();
    Logger::multi_progress().remove(&progress);

    String::from_utf8_lossy(&output.stdout).trim().parse()
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
    fn build(opts: BuildOpts) -> Result<()> {
        impl_build_driver!(build(opts))
    }

    fn tag(opts: TagOpts) -> Result<()> {
        impl_build_driver!(tag(opts))
    }

    fn push(opts: PushOpts) -> Result<()> {
        impl_build_driver!(push(opts))
    }

    fn login(server: &str) -> Result<()> {
        impl_build_driver!(login(server))
    }

    fn prune(opts: PruneOpts) -> Result<()> {
        impl_build_driver!(prune(opts))
    }

    fn build_tag_push(opts: BuildTagPushOpts) -> Result<Vec<String>> {
        impl_build_driver!(build_tag_push(opts))
    }
}

macro_rules! impl_signing_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_signing_driver() {
            SigningDriverType::Cosign => CosignDriver::$func($($args,)*),
            SigningDriverType::Sigstore => SigstoreDriver::$func($($args,)*),
        }
    };
}

impl SigningDriver for Driver {
    fn generate_key_pair(opts: GenerateKeyPairOpts) -> Result<()> {
        impl_signing_driver!(generate_key_pair(opts))
    }

    fn check_signing_files(opts: CheckKeyPairOpts) -> Result<()> {
        impl_signing_driver!(check_signing_files(opts))
    }

    fn sign(opts: SignOpts) -> Result<()> {
        impl_signing_driver!(sign(opts))
    }

    fn verify(opts: VerifyOpts) -> Result<()> {
        impl_signing_driver!(verify(opts))
    }

    fn signing_login(server: &str) -> Result<()> {
        impl_signing_driver!(signing_login(server))
    }
}

impl InspectDriver for Driver {
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata> {
        OciClientDriver::get_metadata(opts)
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
    fn run(opts: RunOpts) -> Result<ExitStatus> {
        impl_run_driver!(run(opts))
    }

    fn run_output(opts: RunOpts) -> Result<Output> {
        impl_run_driver!(run_output(opts))
    }

    fn create_container(opts: CreateContainerOpts) -> Result<types::ContainerId> {
        impl_run_driver!(create_container(opts))
    }

    fn remove_container(opts: RemoveContainerOpts) -> Result<()> {
        impl_run_driver!(remove_container(opts))
    }

    fn remove_image(opts: RemoveImageOpts) -> Result<()> {
        impl_run_driver!(remove_image(opts))
    }

    fn list_images(privileged: bool) -> Result<Vec<Reference>> {
        impl_run_driver!(list_images(privileged))
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

    fn generate_tags(opts: GenerateTagsOpts) -> Result<Vec<String>> {
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

    fn default_ci_file_path() -> std::path::PathBuf {
        impl_ci_driver!(default_ci_file_path())
    }
}

impl ContainerMountDriver for Driver {
    fn mount_container(opts: ContainerOpts) -> Result<types::MountId> {
        PodmanDriver::mount_container(opts)
    }

    fn unmount_container(opts: ContainerOpts) -> Result<()> {
        PodmanDriver::unmount_container(opts)
    }

    fn remove_volume(opts: VolumeOpts) -> Result<()> {
        PodmanDriver::remove_volume(opts)
    }
}

impl OciCopy for Driver {
    fn copy_oci_dir(opts: CopyOciDirOpts) -> Result<()> {
        SkopeoDriver::copy_oci_dir(opts)
    }
}

impl RechunkDriver for Driver {
    fn rechunk(opts: RechunkOpts) -> Result<Vec<String>> {
        PodmanDriver::rechunk(opts)
    }
}

macro_rules! impl_boot_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_boot_driver() {
            #[cfg(feature = "bootc")]
            BootDriverType::Bootc => BootcDriver::$func($($args,)*),
            BootDriverType::RpmOstree => RpmOstreeDriver::$func($($args,)*),
            BootDriverType::None => ::miette::bail!("Cannot perform boot operation when no boot driver exists."),
        }
    };
}

impl BootDriver for Driver {
    fn status() -> Result<Box<dyn BootStatus>> {
        impl_boot_driver!(status())
    }

    fn switch(opts: SwitchOpts) -> Result<()> {
        impl_boot_driver!(switch(opts))
    }

    fn upgrade(opts: SwitchOpts) -> Result<()> {
        impl_boot_driver!(upgrade(opts))
    }
}
