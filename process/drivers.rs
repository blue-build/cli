//! This module is responsible for managing various strategies
//! to perform actions throughout the program.
//!
//! This hides all
//! the implementation details from the command logic and allows
//! for caching certain long execution tasks like inspecting the
//! labels for an image.

use std::{
    borrow::Borrow,
    process::{ExitStatus, Output},
    sync::{LazyLock, RwLock, atomic::AtomicBool},
    time::Duration,
};

use blue_build_utils::{
    BUILD_ID,
    container::{ContainerId, Tag},
    semver::Version,
};
use bon::bon;
use cached::cached;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, trace, warn};
use miette::{Context, Result, bail};
use oci_client::Reference;
use uuid::Uuid;

use crate::{
    ASYNC_RUNTIME,
    drivers::opts::{InspectImageOpts, RemoveImageOpts},
    logging::Logger,
    signal_handler::DetachedContainer,
};
use opts::{
    CheckKeyPairOpts, CreateContainerOpts, GenerateImageNameOpts, GenerateKeyPairOpts,
    GenerateTagsOpts, GetMetadataOpts, RemoveContainerOpts, RunOpts, SignOpts, SwitchOpts,
    VerifyOpts,
};
use types::{BootDriverType, CiDriverType, ImageMetadata};

#[cfg(not(feature = "next-builder"))]
use blue_build_utils::container::{ImageRef, MountId};

pub use self::{
    github_driver::GithubDriver, gitlab_driver::GitlabDriver, local_driver::LocalDriver,
    oci_client_driver::OciClientDriver, podman_driver::PodmanDriver,
    sigstore_driver::SigstoreDriver, traits::*,
};

#[cfg(feature = "bootc")]
pub use bootc_driver::BootcDriver;

#[cfg(feature = "bootc")]
mod bootc_driver;
#[cfg(not(feature = "next-builder"))]
mod buildah_driver;
#[cfg(not(feature = "next-builder"))]
mod cosign_driver;
#[cfg(not(feature = "next-builder"))]
mod docker_driver;
mod github_driver;
mod gitlab_driver;
mod local_driver;
mod oci_client_driver;
pub mod opts;
mod podman_driver;
#[cfg(not(feature = "next-builder"))]
pub mod post_build;
#[cfg(not(feature = "next-builder"))]
mod rpm_ostree_driver;
#[cfg(not(feature = "next-builder"))]
mod rpm_ostree_runner;
mod sigstore_driver;
#[cfg(not(feature = "next-builder"))]
mod skopeo_driver;
mod traits;
pub mod types;

static INIT: AtomicBool = AtomicBool::new(false);
#[cfg(not(feature = "next-builder"))]
static SELECTED_BUILD_DRIVER: LazyLock<RwLock<Option<types::BuildDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
#[cfg(not(feature = "next-builder"))]
static SELECTED_RUN_DRIVER: LazyLock<RwLock<Option<types::RunDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
#[cfg(not(feature = "next-builder"))]
static SELECTED_SIGNING_DRIVER: LazyLock<RwLock<Option<types::SigningDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_CI_DRIVER: LazyLock<RwLock<Option<CiDriverType>>> =
    LazyLock::new(|| RwLock::new(None));
static SELECTED_BOOT_DRIVER: LazyLock<RwLock<Option<BootDriverType>>> =
    LazyLock::new(|| RwLock::new(None));

/// Args for selecting the various drivers to use for runtime.
///
/// If the args are left uninitialized, the program will determine
/// the best one available.
#[cfg(not(feature = "next-builder"))]
#[derive(Default, Clone, Copy, Debug, bon::Builder, clap::Args)]
pub struct DriverArgs {
    /// Select which driver to use to build
    /// your image.
    #[arg(short = 'B', long, env = blue_build_utils::constants::BB_BUILD_DRIVER)]
    pub build_driver: Option<types::BuildDriverType>,

    /// Select which driver to use to inspect
    /// images.
    #[arg(short = 'I', long, env = blue_build_utils::constants::BB_INSPECT_DRIVER)]
    pub inspect_driver: Option<types::InspectDriverType>,

    /// Select which driver to use to sign
    /// images.
    #[arg(short = 'S', long, env = blue_build_utils::constants::BB_SIGNING_DRIVER)]
    pub signing_driver: Option<types::SigningDriverType>,

    /// Select which driver to use to run
    /// containers.
    #[arg(short = 'R', long, env = blue_build_utils::constants::BB_RUN_DRIVER)]
    pub run_driver: Option<types::RunDriverType>,

    #[arg(short = 'T', long, env = blue_build_utils::constants::BB_BOOT_DRIVER)]
    pub boot_driver: Option<types::BootDriverType>,
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
    pub fn init(#[cfg(not(feature = "next-builder"))] mut args: DriverArgs) {
        trace!("Driver::init()");

        #[cfg(not(feature = "next-builder"))]
        if args.inspect_driver.is_some() {
            warn!("Setting the inspect driver is deprecated.");
        }

        #[cfg(not(feature = "next-builder"))]
        impl_driver_init! {
            INIT;
            args.build_driver => SELECTED_BUILD_DRIVER;
            args.run_driver => SELECTED_RUN_DRIVER;
            args.signing_driver => SELECTED_SIGNING_DRIVER;
            args.boot_driver => SELECTED_BOOT_DRIVER;
            default => SELECTED_CI_DRIVER;
        }
        #[cfg(feature = "next-builder")]
        impl_driver_init! {
            INIT;
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

        let os_version = ASYNC_RUNTIME
            .block_on(Self::get_metadata(
                GetMetadataOpts::builder().image(oci_ref).build(),
            ))
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

    #[cfg(not(feature = "next-builder"))]
    pub fn get_build_driver() -> types::BuildDriverType {
        impl_driver_type!(SELECTED_BUILD_DRIVER)
    }

    #[cfg(not(feature = "next-builder"))]
    pub fn get_signing_driver() -> types::SigningDriverType {
        impl_driver_type!(SELECTED_SIGNING_DRIVER)
    }

    #[cfg(not(feature = "next-builder"))]
    pub fn get_run_driver() -> types::RunDriverType {
        impl_driver_type!(SELECTED_RUN_DRIVER)
    }

    pub fn get_ci_driver() -> types::CiDriverType {
        impl_driver_type!(SELECTED_CI_DRIVER)
    }

    pub fn get_boot_driver() -> types::BootDriverType {
        impl_driver_type!(SELECTED_BOOT_DRIVER)
    }

    /// Runs the login logic for the driver.
    ///
    /// # Errors
    /// Will error if login fails.
    #[cfg(feature = "next-builder")]
    pub fn login(server: &str) -> Result<()> {
        PodmanDriver::login(server)
    }

    /// Runs prune commands for the driver.
    ///
    /// # Errors
    /// Will error if the driver fails to prune.
    #[cfg(feature = "next-builder")]
    pub fn prune(opts: opts::PruneOpts) -> Result<()> {
        PodmanDriver::prune(opts)
    }
}

#[cached(
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

    #[cfg(not(feature = "next-builder"))]
    let should_remove = if matches!(Driver::get_run_driver(), types::RunDriverType::Docker) {
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("docker: failed to register layer: max depth exceeded") {
            bail!(
                help = format!(
                    "Try using podman as your container runtime with `{}`",
                    "--run-driver=podman".bright_yellow()
                ),
                "Docker cannot handle images with too many layers!"
            );
        }
        bail!(
            "Failed to run version script on the base image {}:\n{}",
            oci_ref.to_string().cyan(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(not(feature = "next-builder"))]
    if should_remove {
        Driver::remove_image(RemoveImageOpts::builder().image(oci_ref).build())?;
    }

    progress.finish_and_clear();
    Logger::multi_progress().remove(&progress);

    String::from_utf8_lossy(&output.stdout).trim().parse()
}

#[cfg(not(feature = "next-builder"))]
macro_rules! impl_build_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_build_driver() {
            types::BuildDriverType::Buildah => buildah_driver::BuildahDriver::$func($($args,)*),
            types::BuildDriverType::Podman => PodmanDriver::$func($($args,)*),
            types::BuildDriverType::Docker => docker_driver::DockerDriver::$func($($args,)*),
        }
    };
}

impl ImageStorageDriver for Driver {
    fn inspect_image(opts: InspectImageOpts) -> Result<Option<Vec<u8>>> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_build_driver!(inspect_image(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::inspect_image(opts)
        }
    }

    fn remove_image(opts: RemoveImageOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_build_driver!(remove_image(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::remove_image(opts)
        }
    }

    fn list_images(privileged: bool) -> Result<Vec<Reference>> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_build_driver!(list_images(privileged))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::list_images(privileged)
        }
    }
}

#[cfg(not(feature = "next-builder"))]
impl BuildDriver for Driver {
    fn build(opts: opts::BuildOpts) -> Result<()> {
        impl_build_driver!(build(opts))
    }

    fn tag(opts: opts::TagOpts) -> Result<()> {
        impl_build_driver!(tag(opts))
    }

    fn untag(opts: opts::UntagOpts) -> Result<()> {
        impl_build_driver!(untag(opts))
    }

    fn push(opts: opts::PushOpts) -> Result<()> {
        impl_build_driver!(push(opts))
    }

    fn pull(opts: opts::PullOpts) -> Result<ContainerId> {
        impl_build_driver!(pull(opts))
    }

    fn login(server: &str) -> Result<()> {
        impl_build_driver!(login(server))
    }

    fn prune(opts: opts::PruneOpts) -> Result<()> {
        impl_build_driver!(prune(opts))
    }

    fn manifest_create(opts: opts::ManifestCreateOpts) -> Result<()> {
        impl_build_driver!(manifest_create(opts))
    }

    fn manifest_push(opts: opts::ManifestPushOpts) -> Result<()> {
        impl_build_driver!(manifest_push(opts))
    }

    fn build_tag_push(opts: opts::BuildTagPushOpts) -> Result<Vec<String>> {
        impl_build_driver!(build_tag_push(opts))
    }
}

#[cfg(not(feature = "next-builder"))]
impl PostBuildDriver for Driver {
    fn build_tag_push_with_post_build(
        opts: opts::BuildTagPushOpts,
        pb_opts: opts::PostBuildDriverOpts,
    ) -> Result<Vec<String>> {
        PodmanDriver::build_tag_push_with_post_build(opts, pb_opts)
    }
}

#[cfg(not(feature = "next-builder"))]
macro_rules! impl_signing_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_signing_driver() {
            types::SigningDriverType::Cosign => cosign_driver::CosignDriver::$func($($args,)*),
            types::SigningDriverType::Sigstore => sigstore_driver::SigstoreDriver::$func($($args,)*),
        }
    };
}

impl SigningDriver for Driver {
    fn generate_key_pair(opts: GenerateKeyPairOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_signing_driver!(generate_key_pair(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            SigstoreDriver::generate_key_pair(opts)
        }
    }

    fn check_signing_files(opts: CheckKeyPairOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_signing_driver!(check_signing_files(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            SigstoreDriver::check_signing_files(opts)
        }
    }

    fn sign(opts: SignOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_signing_driver!(sign(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            SigstoreDriver::sign(opts)
        }
    }

    fn verify(opts: VerifyOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_signing_driver!(verify(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            SigstoreDriver::verify(opts)
        }
    }

    fn signing_login(server: &str) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_signing_driver!(signing_login(server))
        }
        #[cfg(feature = "next-builder")]
        {
            SigstoreDriver::signing_login(server)
        }
    }
}

impl InspectDriver for Driver {
    async fn get_metadata(opts: GetMetadataOpts<'_>) -> Result<ImageMetadata> {
        OciClientDriver::get_metadata(opts).await
    }
}

#[cfg(not(feature = "next-builder"))]
macro_rules! impl_run_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_run_driver() {
            types::RunDriverType::Docker => docker_driver::DockerDriver::$func($($args,)*),
            types::RunDriverType::Podman => PodmanDriver::$func($($args,)*),
        }
    };
}

impl RunDriver for Driver {
    fn run(opts: RunOpts) -> Result<ExitStatus> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_run_driver!(run(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::run(opts)
        }
    }

    fn run_output(opts: RunOpts) -> Result<Output> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_run_driver!(run_output(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::run_output(opts)
        }
    }

    fn run_detached(opts: RunOpts) -> Result<DetachedContainer> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_run_driver!(run_detached(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::run_detached(opts)
        }
    }

    fn create_container(opts: CreateContainerOpts) -> Result<ContainerId> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_run_driver!(create_container(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::create_container(opts)
        }
    }

    fn remove_container(opts: RemoveContainerOpts) -> Result<()> {
        #[cfg(not(feature = "next-builder"))]
        {
            impl_run_driver!(remove_container(opts))
        }
        #[cfg(feature = "next-builder")]
        {
            PodmanDriver::remove_container(opts)
        }
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

    fn generate_tags(opts: GenerateTagsOpts) -> Result<Vec<Tag>> {
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

#[cfg(not(feature = "next-builder"))]
impl BuildChunkedOciDriver for Driver {
    fn manifest_create_with_runner(
        runner: &rpm_ostree_runner::RpmOstreeRunner,
        opts: opts::ManifestCreateOpts,
    ) -> Result<()> {
        PodmanDriver::manifest_create_with_runner(runner, opts)
    }

    fn manifest_push_with_runner(
        runner: &rpm_ostree_runner::RpmOstreeRunner,
        opts: opts::ManifestPushOpts,
    ) -> Result<()> {
        PodmanDriver::manifest_push_with_runner(runner, opts)
    }

    fn pull_with_runner(
        runner: &rpm_ostree_runner::RpmOstreeRunner,
        opts: opts::PullOpts,
    ) -> Result<ContainerId> {
        PodmanDriver::pull_with_runner(runner, opts)
    }

    fn remove_image_with_runner(
        runner: &rpm_ostree_runner::RpmOstreeRunner,
        image_ref: &str,
    ) -> Result<()> {
        PodmanDriver::remove_image_with_runner(runner, image_ref)
    }

    fn build_chunked_oci(
        runner: &rpm_ostree_runner::RpmOstreeRunner,
        unchunked_image: &ImageRef<'_>,
        final_image: &ImageRef<'_>,
        opts: opts::BuildChunkedOciOpts,
    ) -> Result<()> {
        PodmanDriver::build_chunked_oci(runner, unchunked_image, final_image, opts)
    }

    fn build_rechunk_tag_push(opts: opts::BuildRechunkTagPushOpts) -> Result<Vec<String>> {
        PodmanDriver::build_rechunk_tag_push(opts)
    }
}

#[cfg(not(feature = "next-builder"))]
impl ContainerMountDriver for Driver {
    fn mount_container(opts: opts::ContainerOpts) -> Result<MountId> {
        PodmanDriver::mount_container(opts)
    }

    fn unmount_container(opts: opts::ContainerOpts) -> Result<()> {
        PodmanDriver::unmount_container(opts)
    }

    fn remove_volume(opts: opts::VolumeOpts) -> Result<()> {
        PodmanDriver::remove_volume(opts)
    }
}

#[cfg(not(feature = "next-builder"))]
impl OciCopy for Driver {
    fn copy_oci(&self, opts: opts::CopyOciOpts) -> Result<()> {
        skopeo_driver::SkopeoDriver.copy_oci(opts)
    }
}

#[cfg(not(feature = "next-builder"))]
impl RechunkDriver for Driver {
    fn rechunk(opts: opts::RechunkOpts) -> Result<Vec<String>> {
        PodmanDriver::rechunk(opts)
    }
}

macro_rules! impl_boot_driver {
    ($func:ident($($args:expr),*)) => {
        match Self::get_boot_driver() {
            #[cfg(feature = "bootc")]
            BootDriverType::Bootc => BootcDriver::$func($($args,)*),
            #[cfg(not(feature = "next-builder"))]
            BootDriverType::RpmOstree => rpm_ostree_driver::RpmOstreeDriver::$func($($args,)*),
            BootDriverType::None => {
                ::miette::bail!("Cannot perform boot operation when no boot driver exists.");
            },
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
