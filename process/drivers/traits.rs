use std::{
    borrow::Borrow,
    ops::Not,
    path::PathBuf,
    process::{ExitStatus, Output},
};

use blue_build_utils::{
    constants::COSIGN_PUB_PATH,
    container::{ContainerId, ImageRef, MountId, OciRef, Tag},
    platform::Platform,
    retry,
    semver::Version,
    string_vec,
};
use comlexr::cmd;
use log::{debug, info, trace, warn};
use miette::{Context, IntoDiagnostic, Result, bail};
use oci_client::Reference;
use rayon::prelude::*;
use semver::VersionReq;

use super::{
    Driver,
    opts::{
        BuildChunkedOciOpts, BuildOpts, BuildRechunkTagPushOpts, BuildTagPushOpts,
        CheckKeyPairOpts, ContainerOpts, CopyOciOpts, CreateContainerOpts, GenerateImageNameOpts,
        GenerateKeyPairOpts, GenerateTagsOpts, GetMetadataOpts, PruneOpts, PullOpts, PushOpts,
        RechunkOpts, RemoveContainerOpts, RemoveImageOpts, RunOpts, SignOpts, SignVerifyOpts,
        SwitchOpts, TagOpts, UntagOpts, VerifyOpts, VerifyType, VolumeOpts,
    },
    opts::{ManifestCreateOpts, ManifestPushOpts},
    rpm_ostree_runner::RpmOstreeRunner,
    types::CiDriverType,
    types::{
        BootDriverType, BuildDriverType, ImageMetadata, InspectDriverType, RunDriverType,
        SigningDriverType,
    },
};
use crate::{
    drivers::opts::PrivateKey, logging::CommandLogging, signal_handler::DetachedContainer,
};

trait PrivateDriver {}

macro_rules! impl_private_driver {
    ($($driver:ty),* $(,)?) => {
        $(
            impl PrivateDriver for $driver {}
        )*
    };
}

impl_private_driver!(
    super::Driver,
    super::docker_driver::DockerDriver,
    super::podman_driver::PodmanDriver,
    super::buildah_driver::BuildahDriver,
    super::github_driver::GithubDriver,
    super::gitlab_driver::GitlabDriver,
    super::local_driver::LocalDriver,
    super::cosign_driver::CosignDriver,
    super::skopeo_driver::SkopeoDriver,
    super::sigstore_driver::SigstoreDriver,
    super::rpm_ostree_driver::RpmOstreeDriver,
    super::rpm_ostree_driver::Status,
    super::rpm_ostree_runner::RpmOstreeContainer,
    super::rpm_ostree_runner::RpmOstreeRunner,
    super::oci_client_driver::OciClientDriver,
    Option<BuildDriverType>,
    Option<RunDriverType>,
    Option<InspectDriverType>,
    Option<SigningDriverType>,
    Option<CiDriverType>,
    Option<BootDriverType>,
);

#[cfg(feature = "bootc")]
impl_private_driver!(
    super::bootc_driver::BootcDriver,
    super::bootc_driver::BootcStatus
);

#[expect(private_bounds)]
pub trait DetermineDriver<T>: PrivateDriver {
    fn determine_driver(&mut self) -> T;
}

/// Trait for retrieving version of a driver.
#[expect(private_bounds)]
pub trait DriverVersion: PrivateDriver {
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

/// Allows agnostic building, tagging, pushing, and login.
#[expect(private_bounds)]
pub trait BuildDriver: PrivateDriver {
    /// Runs the build logic for the driver.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn build(opts: BuildOpts) -> Result<()>;

    /// Runs the tag logic for the driver.
    ///
    /// # Errors
    /// Will error if the tagging fails.
    fn tag(opts: TagOpts) -> Result<()>;

    /// Runs the untag logic for the driver.
    ///
    /// # Errors
    /// Will error if the untagging fails.
    fn untag(opts: UntagOpts) -> Result<()>;

    /// Runs the push logic for the driver
    ///
    /// # Errors
    /// Will error if the push fails.
    fn push(opts: PushOpts) -> Result<()>;

    /// Runs the pull logic for the driver
    ///
    /// # Errors
    /// Will error if the pull fails.
    fn pull(opts: PullOpts) -> Result<ContainerId>;

    /// Runs the login logic for the driver.
    ///
    /// # Errors
    /// Will error if login fails.
    fn login(server: &str) -> Result<()>;

    /// Runs prune commands for the driver.
    ///
    /// # Errors
    /// Will error if the driver fails to prune.
    fn prune(opts: super::opts::PruneOpts) -> Result<()>;

    /// Create a manifest containing all the built images.
    ///
    /// # Errors
    /// Will error if the driver fails to create a manifest.
    fn manifest_create(opts: ManifestCreateOpts) -> Result<()>;

    /// Pushes a manifest containing all the built images.
    ///
    /// # Errors
    /// Will error if the driver fails to push a manifest.
    fn manifest_push(opts: ManifestPushOpts) -> Result<()>;

    /// Runs the logic for building, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, tagging, or pushing fails.
    fn build_tag_push(opts: BuildTagPushOpts) -> Result<Vec<String>> {
        trace!("BuildDriver::build_tag_push({opts:#?})");

        assert!(
            opts.platform.is_empty().not(),
            "Must have at least 1 platform"
        );
        let platform_images: Vec<(ImageRef<'_>, Platform)> = opts
            .platform
            .iter()
            .map(|&platform| (opts.image.with_platform(platform), platform))
            .collect();

        let build_opts = BuildOpts::builder()
            .containerfile(opts.containerfile.as_ref())
            .squash(opts.squash)
            .maybe_cache_from(opts.cache_from)
            .maybe_cache_to(opts.cache_to)
            .secrets(opts.secrets);
        let build_opts = platform_images
            .iter()
            .map(|(image, platform)| build_opts.clone().image(image).platform(*platform).build())
            .collect::<Vec<_>>();

        build_opts
            .par_iter()
            .try_for_each(|&build_opts| -> Result<()> {
                info!("Building image {}", build_opts.image);

                Self::build(build_opts)
            })?;

        let image_list: Vec<String> = match &opts.image {
            ImageRef::Remote(image) if !opts.tags.is_empty() => {
                debug!("Tagging all images");

                let mut image_list = Vec::with_capacity(opts.tags.len());
                let platform_images = opts
                    .platform
                    .iter()
                    .map(|&platform| platform.tagged_image(image))
                    .collect::<Vec<_>>();

                for tag in opts.tags {
                    debug!("Tagging {} with {tag}", &image);
                    let tagged_image = Reference::with_tag(
                        image.registry().into(),
                        image.repository().into(),
                        tag.to_string(),
                    );

                    Self::manifest_create(
                        ManifestCreateOpts::builder()
                            .final_image(&tagged_image)
                            .image_list(&platform_images)
                            .build(),
                    )?;
                    image_list.push(tagged_image.to_string());

                    if opts.push {
                        let retry_count = if opts.retry_push { opts.retry_count } else { 0 };

                        // Push images with retries (1s delay between retries)
                        blue_build_utils::retry(retry_count, 5, || {
                            debug!("Pushing image {tagged_image}");

                            Self::manifest_push(
                                ManifestPushOpts::builder()
                                    .final_image(&tagged_image)
                                    .compression_type(opts.compression)
                                    .build(),
                            )
                        })?;
                    }
                }

                image_list
            }
            _ => {
                string_vec![opts.image]
            }
        };

        Ok(image_list)
    }
}

/// Allows agnostic inspection of images.
#[expect(private_bounds)]
pub trait InspectDriver: PrivateDriver {
    /// Gets the metadata on an image tag.
    ///
    /// # Errors
    /// Will error if it is unable to get the labels.
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata>;
}

/// Allows agnostic running of containers.
pub trait RunDriver: ImageStorageDriver {
    /// Run a container to perform an action.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run(opts: RunOpts) -> Result<ExitStatus>;

    /// Run a container to perform an action and capturing output.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run_output(opts: RunOpts) -> Result<Output>;

    /// Run a container to perform an action in the background.
    /// The container will be stopped when the returned `DetachedContainer`
    /// value is dropped.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run_detached(opts: RunOpts) -> Result<DetachedContainer>;

    /// Creates container
    ///
    /// # Errors
    /// Will error if the container create command fails.
    fn create_container(opts: CreateContainerOpts) -> Result<ContainerId>;

    /// Removes a container
    ///
    /// # Errors
    /// Will error if the container remove command fails.
    fn remove_container(opts: RemoveContainerOpts) -> Result<()>;
}

/// Allows agnostic management of container image storage.
#[expect(private_bounds)]
pub trait ImageStorageDriver: PrivateDriver {
    /// Removes an image
    ///
    /// # Errors
    /// Will error if the image remove command fails.
    fn remove_image(opts: RemoveImageOpts) -> Result<()>;

    /// List all images in the local image registry.
    ///
    /// # Errors
    /// Will error if the image list command fails.
    fn list_images(privileged: bool) -> Result<Vec<Reference>>;
}

pub trait BuildChunkedOciDriver: BuildDriver + ImageStorageDriver {
    /// Create a manifest containing all the built images.
    /// Runs within the same context as rpm-ostree.
    ///
    /// # Errors
    /// Will error if the driver fails to create a manifest.
    fn manifest_create_with_runner(
        runner: &RpmOstreeRunner,
        opts: ManifestCreateOpts,
    ) -> Result<()>;

    /// Pushes a manifest containing all the built images.
    /// Runs within the same context as rpm-ostree.
    ///
    /// # Errors
    /// Will error if the driver fails to push a manifest.
    fn manifest_push_with_runner(runner: &RpmOstreeRunner, opts: ManifestPushOpts) -> Result<()>;

    /// Pull an image from a remote registry.
    /// Runs within the same context as rpm-ostree.
    ///
    /// # Errors
    /// Will error if the driver fails to pull the image.
    fn pull_with_runner(runner: &RpmOstreeRunner, opts: PullOpts) -> Result<ContainerId>;

    /// Removes an image from local storage.
    /// Runs within the same context as rpm-ostree.
    ///
    /// # Errors
    /// Will error if image removal fails.
    fn remove_image_with_runner(runner: &RpmOstreeRunner, image_ref: &str) -> Result<()>;

    /// Runs build-chunked-oci on an image.
    ///
    /// # Errors
    /// Will error if rechunking fails.
    fn build_chunked_oci(
        runner: &RpmOstreeRunner,
        unchunked_image: &ImageRef<'_>,
        final_image: &ImageRef<'_>,
        opts: BuildChunkedOciOpts,
    ) -> Result<()> {
        trace!(
            concat!(
                "BuildChunkedOciDriver::build_chunked_oci(\n",
                "runner: {:#?},\n",
                "unchunked_image: {},\n",
                "final_image: {},\n",
                "opts: {:#?})\n)"
            ),
            runner, unchunked_image, final_image, opts,
        );

        let prev_image_id = if !opts.clear_plan
            && let ImageRef::Remote(image_ref) = final_image
        {
            Self::pull_with_runner(
                runner,
                PullOpts::builder()
                    .image(image_ref)
                    .maybe_platform(opts.platform)
                    .retry_count(5)
                    .build(),
            )
            .inspect_err(|_| {
                warn!("Failed to pull previous build; rechunking will use fresh layer plan.");
            })
            .ok()
        } else {
            None
        };

        let (first_cmd, args) =
            runner.command_args("rpm-ostree", &["compose", "build-chunked-oci"]);
        let transport_ref = match final_image {
            ImageRef::Remote(image) => format!("containers-storage:{image}"),
            _ => final_image.to_string(),
        };
        let command = cmd!(
            first_cmd,
            for args,
            "--bootc",
            format!("--format-version={}", opts.format_version),
            format!("--max-layers={}", opts.max_layers),
            format!("--from={unchunked_image}"),
            format!("--output={transport_ref}"),
        );
        trace!("{command:?}");
        let status = command
            .build_status(final_image.to_string(), "Rechunking image")
            .into_diagnostic()?;

        if let Some(image_id) = prev_image_id {
            Self::remove_image_with_runner(runner, &image_id.0)?;
        }

        if !status.success() {
            bail!("Failed to rechunk image {}", final_image);
        }

        Ok(())
    }

    /// Runs the logic for building, rechunking, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, rechunking, tagging, or pushing fails.
    #[expect(clippy::too_many_lines)]
    fn build_rechunk_tag_push(opts: BuildRechunkTagPushOpts) -> Result<Vec<String>> {
        trace!("BuildChunkedOciDriver::build_rechunk_tag_push({opts:#?})");

        let BuildRechunkTagPushOpts {
            build_tag_push_opts: btp_opts,
            rechunk_opts,
            remove_base_image,
        } = opts;

        assert!(
            btp_opts.platform.is_empty().not(),
            "Must have at least 1 platform"
        );
        let build_opts = BuildOpts::builder()
            .containerfile(btp_opts.containerfile.as_ref())
            .squash(true)
            .secrets(btp_opts.secrets);

        let images_to_rechunk: Vec<(ImageRef, ImageRef, Platform)> = btp_opts
            .platform
            .par_iter()
            .map(|&platform| -> Result<(ImageRef, ImageRef, Platform)> {
                let image = btp_opts.image.with_platform(platform);
                let unchunked_image =
                    image.append_tag(&"unchunked".parse().expect("Should be a valid tag"));
                info!("Building image {image}");

                Self::build(
                    build_opts
                        .clone()
                        .image(&unchunked_image)
                        .platform(platform)
                        .build(),
                )?;
                Ok((unchunked_image, image, platform))
            })
            .collect::<Result<Vec<_>>>()?;

        if let Some(base_image) = remove_base_image {
            Self::remove_image(
                RemoveImageOpts::builder()
                    .image(base_image)
                    .privileged(btp_opts.privileged)
                    .build(),
            )?;
            Self::prune(PruneOpts::builder().volumes(true).build())?;
        }

        // Run subsequent commands on host if rpm-ostree is available on host, otherwise
        // run in container that has rpm-ostree installed.
        let runner = RpmOstreeRunner::start()?;

        // Rechunk images serially to avoid using excessive disk space.
        if let ImageRef::Remote(image_ref) = btp_opts.image {
            for (unchunked_image, image, platform) in images_to_rechunk {
                // Use the non-platform-tagged image ref as the output for build-chunked-oci
                // so it looks for an existing manifest at the right location (the multi-arch
                // image that will be pushed). This allows build-chunked-oci to read the
                // previous build's layer annotations to minimize layout changes.
                let result = Self::build_chunked_oci(
                    &runner,
                    &unchunked_image,
                    btp_opts.image,
                    rechunk_opts.with_platform(platform),
                );
                // Clean up the unchunked image whether or not rechunking succeeded.
                if let ImageRef::Remote(unchunked_image) = unchunked_image {
                    Self::remove_image(RemoveImageOpts::builder().image(&unchunked_image).build())?;
                }
                result?;

                // Now retag the image to use the platform tag.
                if let ImageRef::Remote(image_with_platform) = image {
                    Self::tag(
                        TagOpts::builder()
                            .src_image(image_ref)
                            .dest_image(&image_with_platform)
                            .privileged(btp_opts.privileged)
                            .build(),
                    )?;
                    Self::untag(
                        UntagOpts::builder()
                            .image(image_ref)
                            .privileged(btp_opts.privileged)
                            .build(),
                    )?;
                }
            }
        } else {
            for (unchunked_image, image, platform) in images_to_rechunk {
                Self::build_chunked_oci(
                    &runner,
                    &unchunked_image,
                    &image,
                    rechunk_opts.with_platform(platform),
                )?;
            }
        }

        let image_list: Vec<String> = match &btp_opts.image {
            ImageRef::Remote(image) if !btp_opts.tags.is_empty() => {
                debug!("Tagging all images");

                let mut image_list = Vec::with_capacity(btp_opts.tags.len());
                let platform_images = btp_opts
                    .platform
                    .iter()
                    .map(|&platform| platform.tagged_image(image))
                    .collect::<Vec<_>>();

                for tag in btp_opts.tags {
                    debug!("Tagging {} with {tag}", &image);
                    let tagged_image = Reference::with_tag(
                        image.registry().into(),
                        image.repository().into(),
                        tag.to_string(),
                    );

                    Self::manifest_create_with_runner(
                        &runner,
                        ManifestCreateOpts::builder()
                            .final_image(&tagged_image)
                            .image_list(&platform_images)
                            .build(),
                    )?;
                    image_list.push(tagged_image.to_string());

                    if btp_opts.push {
                        let retry_count = if btp_opts.retry_push {
                            btp_opts.retry_count
                        } else {
                            0
                        };

                        // Push images with retries (1s delay between retries)
                        blue_build_utils::retry(retry_count, 5, || {
                            debug!("Pushing image {tagged_image}");

                            // We push twice due to a (very strange) bug in podman where layer
                            // annotations aren't pushed unless the layer already exists in the
                            // remote registry. See:
                            // https://github.com/containers/podman/issues/27796
                            Self::manifest_push_with_runner(
                                &runner,
                                ManifestPushOpts::builder()
                                    .final_image(&tagged_image)
                                    .compression_type(btp_opts.compression)
                                    .build(),
                            )
                            .and_then(|()| {
                                Self::manifest_push_with_runner(
                                    &runner,
                                    ManifestPushOpts::builder()
                                        .final_image(&tagged_image)
                                        .compression_type(btp_opts.compression)
                                        .build(),
                                )
                            })
                        })?;
                    }
                }

                image_list
            }
            _ => {
                string_vec![btp_opts.image]
            }
        };

        Ok(image_list)
    }
}

#[expect(private_bounds)]
pub(super) trait ContainerMountDriver: PrivateDriver {
    /// Mounts the container
    ///
    /// # Errors
    /// Will error if the container mount command fails.
    fn mount_container(opts: ContainerOpts) -> Result<MountId>;

    /// Unmount the container
    ///
    /// # Errors
    /// Will error if the container unmount command fails.
    fn unmount_container(opts: ContainerOpts) -> Result<()>;

    /// Remove a volume
    ///
    /// # Errors
    /// Will error if the volume remove command fails.
    fn remove_volume(opts: VolumeOpts) -> Result<()>;
}

#[expect(private_bounds)]
pub trait OciCopy: PrivateDriver {
    /// Copy an OCI image.
    ///
    /// # Errors
    /// Will error if copying the image fails.
    fn copy_oci(&self, opts: CopyOciOpts) -> Result<()>;
}

#[expect(private_bounds)]
pub trait RechunkDriver: RunDriver + BuildDriver + ContainerMountDriver {
    const RECHUNK_IMAGE: &str = "ghcr.io/hhd-dev/rechunk:v1.0.1";

    /// Perform a rechunk build of a recipe.
    ///
    /// # Errors
    /// Will error if the rechunk process fails.
    fn rechunk(opts: RechunkOpts) -> Result<Vec<String>> {
        assert!(
            opts.platform.is_empty().not(),
            "Must have at least one platform defined!"
        );

        let temp_dir = if let Some(dir) = opts.tempdir {
            &tempfile::TempDir::new_in(dir).into_diagnostic()?
        } else {
            &tempfile::TempDir::new().into_diagnostic()?
        };
        let ostree_cache_id = &uuid::Uuid::new_v4().to_string();
        let image = &ImageRef::from(
            Reference::try_from(format!("localhost/{ostree_cache_id}/raw-rechunk")).unwrap(),
        );
        let current_dir = &std::env::current_dir().into_diagnostic()?;
        let current_dir = &*current_dir.to_string_lossy();
        let main_tag = opts.tags.first().cloned().unwrap_or_default();
        let final_image = Reference::with_tag(
            opts.image.resolve_registry().into(),
            opts.image.repository().into(),
            main_tag.as_str().into(),
        );

        Self::login(final_image.registry())?;

        let platform_images = opts
            .platform
            .iter()
            .map(|&platform| (image.with_platform(platform), platform))
            .collect::<Vec<_>>();
        let build_opts = platform_images
            .iter()
            .map(|(image, platform)| {
                BuildOpts::builder()
                    .image(image)
                    .containerfile(opts.containerfile)
                    .platform(*platform)
                    .privileged(true)
                    .squash(true)
                    .host_network(true)
                    .secrets(opts.secrets)
                    .build()
            })
            .collect::<Vec<_>>();

        build_opts.par_iter().try_for_each(|&build_opts| {
            let ImageRef::Remote(image) = build_opts.image else {
                bail!("Cannot build for {}", build_opts.image);
            };
            Self::build(build_opts)?;
            let container = &Self::create_container(
                CreateContainerOpts::builder()
                    .image(image)
                    .privileged(true)
                    .build(),
            )?;
            let mount = &Self::mount_container(
                ContainerOpts::builder()
                    .container_id(container)
                    .privileged(true)
                    .build(),
            )?;

            Self::prune_image(mount, container, image, opts)?;
            Self::create_ostree_commit(mount, ostree_cache_id, container, image, opts)?;

            let temp_dir_str = &*temp_dir.path().to_string_lossy();

            Self::rechunk_image(ostree_cache_id, temp_dir_str, current_dir, opts)
        })?;

        let mut image_list = Vec::with_capacity(opts.tags.len());

        if opts.push {
            let oci_dir = OciRef::from_oci_directory(temp_dir.path().join(ostree_cache_id))?;

            for tag in opts.tags {
                let tagged_image = Reference::with_tag(
                    final_image.registry().to_string(),
                    final_image.repository().to_string(),
                    tag.to_string(),
                );

                blue_build_utils::retry(opts.retry_count, 5, || {
                    debug!("Pushing image {tagged_image}");

                    Driver.copy_oci(
                        CopyOciOpts::builder()
                            .src_ref(&oci_dir)
                            .dest_ref(&OciRef::from_remote_ref(&tagged_image))
                            .privileged(true)
                            .build(),
                    )
                })?;
                image_list.push(tagged_image.into());
            }
        }

        Ok(image_list)
    }

    /// Step 1 of the rechunk process that prunes excess files.
    ///
    /// # Errors
    /// Will error if the prune process fails.
    fn prune_image(
        mount: &MountId,
        container: &ContainerId,
        image: &Reference,
        opts: RechunkOpts<'_>,
    ) -> Result<(), miette::Error> {
        let status = Self::run(
            RunOpts::builder()
                .image(Self::RECHUNK_IMAGE)
                .remove(true)
                .user("0:0")
                .privileged(true)
                .volumes(&crate::run_volumes! {
                    mount => "/var/tree",
                })
                .env_vars(&crate::run_envs! {
                    "TREE" => "/var/tree",
                })
                .args(&bon::vec!["/sources/rechunk/1_prune.sh"])
                .build(),
        )?;

        if !status.success() {
            Self::unmount_container(
                super::opts::ContainerOpts::builder()
                    .container_id(container)
                    .privileged(true)
                    .build(),
            )?;
            Self::remove_container(
                RemoveContainerOpts::builder()
                    .container_id(container)
                    .privileged(true)
                    .build(),
            )?;
            Self::remove_image(
                RemoveImageOpts::builder()
                    .image(image)
                    .privileged(true)
                    .build(),
            )?;
            bail!("Failed to run prune step for {}", &opts.image);
        }

        Ok(())
    }

    /// Step 2 of the rechunk process that creates the ostree commit.
    ///
    /// # Errors
    /// Will error if the ostree commit process fails.
    fn create_ostree_commit(
        mount: &MountId,
        ostree_cache_id: &str,
        container: &ContainerId,
        image: &Reference,
        opts: RechunkOpts<'_>,
    ) -> Result<()> {
        let status = Self::run(
            RunOpts::builder()
                .image(Self::RECHUNK_IMAGE)
                .remove(true)
                .user("0:0")
                .privileged(true)
                .volumes(&crate::run_volumes! {
                    mount => "/var/tree",
                    ostree_cache_id => "/var/ostree",
                })
                .env_vars(&crate::run_envs! {
                    "TREE" => "/var/tree",
                    "REPO" => "/var/ostree/repo",
                    "RESET_TIMESTAMP" => "1",
                })
                .args(&bon::vec!["/sources/rechunk/2_create.sh"])
                .build(),
        )?;
        Self::unmount_container(
            super::opts::ContainerOpts::builder()
                .container_id(container)
                .privileged(true)
                .build(),
        )?;
        Self::remove_container(
            RemoveContainerOpts::builder()
                .container_id(container)
                .privileged(true)
                .build(),
        )?;
        Self::remove_image(
            RemoveImageOpts::builder()
                .image(image)
                .privileged(true)
                .build(),
        )?;

        if !status.success() {
            bail!("Failed to run Ostree create step for {}", &opts.image);
        }

        Ok(())
    }

    /// Step 3 of the rechunk process that generates the final chunked image.
    ///
    /// # Errors
    /// Will error if the chunk process fails.
    fn rechunk_image(
        ostree_cache_id: &str,
        temp_dir_str: &str,
        current_dir: &str,
        opts: RechunkOpts<'_>,
    ) -> Result<()> {
        let out_ref = format!("oci:{ostree_cache_id}");
        let image = opts.image.to_string();
        let label_string = opts
            .labels
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .reduce(|a, b| format!("{a}\n{b}"))
            .unwrap_or_default();
        let status = Self::run(
            RunOpts::builder()
                .image(Self::RECHUNK_IMAGE)
                .remove(true)
                .user("0:0")
                .privileged(true)
                .volumes(&crate::run_volumes! {
                    ostree_cache_id => "/var/ostree",
                    temp_dir_str => "/workspace",
                    current_dir => "/var/git"
                })
                .env_vars(&crate::run_envs! {
                    "REPO" => "/var/ostree/repo",
                    "PREV_REF" => &image,
                    "OUT_NAME" => ostree_cache_id,
                    "CLEAR_PLAN" => if opts.clear_plan { "true" } else { "" },
                    "VERSION" => opts.version,
                    "OUT_REF" => &out_ref,
                    "GIT_DIR" => "/var/git",
                    "LABELS" => &label_string,
                })
                .args(&bon::vec!["/sources/rechunk/3_chunk.sh"])
                .build(),
        )?;

        Self::remove_volume(
            super::opts::VolumeOpts::builder()
                .volume_id(ostree_cache_id)
                .privileged(true)
                .build(),
        )?;

        if !status.success() {
            bail!("Failed to run rechunking for {}", &opts.image);
        }

        Ok(())
    }
}

/// Allows agnostic management of signature keys.
#[expect(private_bounds)]
pub trait SigningDriver: PrivateDriver {
    /// Generate a new private/public key pair.
    ///
    /// # Errors
    /// Will error if a key-pair couldn't be generated.
    fn generate_key_pair(opts: GenerateKeyPairOpts) -> Result<()>;

    /// Checks the signing key files to ensure
    /// they match.
    ///
    /// # Errors
    /// Will error if the files cannot be verified.
    fn check_signing_files(opts: CheckKeyPairOpts) -> Result<()>;

    /// Signs the image digest.
    ///
    /// # Errors
    /// Will error if signing fails.
    fn sign(opts: SignOpts) -> Result<()>;

    /// Verifies the image.
    ///
    /// The image can be verified either with `VerifyType::File` containing
    /// the public key contents, or with `VerifyType::Keyless` containing
    /// information about the `issuer` and `identity`.
    ///
    /// # Errors
    /// Will error if the image fails to be verified.
    fn verify(opts: VerifyOpts) -> Result<()>;

    /// Sign an image given the image name and tag.
    ///
    /// # Errors
    /// Will error if the image fails to be signed.
    fn sign_and_verify(opts: SignVerifyOpts) -> Result<()> {
        trace!("sign_and_verify({opts:?})");

        let path = opts
            .dir
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |d| d.to_path_buf());
        let cosign_file_path = path.join(COSIGN_PUB_PATH);

        let metadata = Driver::get_metadata(
            GetMetadataOpts::builder()
                .image(opts.image)
                .no_cache(true)
                .build(),
        )?;
        let image_digest = Reference::with_digest(
            opts.image.resolve_registry().into(),
            opts.image.repository().into(),
            metadata.digest().into(),
        );
        let issuer = Driver::oidc_provider();
        let identity = Driver::keyless_cert_identity();
        let priv_key = PrivateKey::new(&path);

        let (sign_opts, verify_opts) =
            match (Driver::get_ci_driver(), &priv_key, &issuer, &identity) {
                // Cosign public/private key pair
                (_, Ok(priv_key), _, _) => (
                    SignOpts::builder()
                        .image(&image_digest)
                        .key(priv_key)
                        .metadata(&metadata)
                        .build(),
                    VerifyOpts::builder()
                        .image(&image_digest)
                        .verify_type(VerifyType::File(&cosign_file_path))
                        .build(),
                ),
                // Gitlab keyless
                (CiDriverType::Github | CiDriverType::Gitlab, _, Ok(issuer), Ok(identity)) => (
                    SignOpts::builder()
                        .metadata(&metadata)
                        .image(&image_digest)
                        .build(),
                    VerifyOpts::builder()
                        .image(&image_digest)
                        .verify_type(VerifyType::Keyless { issuer, identity })
                        .build(),
                ),
                _ => bail!("Failed to get information for signing the image"),
            };

        let retry_count = if opts.retry_push { opts.retry_count } else { 0 };

        retry(retry_count, 5, || {
            Self::sign(sign_opts)?;
            Self::verify(verify_opts)
        })?;

        Ok(())
    }

    /// Runs the login logic for the signing driver.
    ///
    /// # Errors
    /// Will error if login fails.
    fn signing_login(server: &str) -> Result<()>;
}

/// Allows agnostic retrieval of CI-based information.
#[expect(private_bounds)]
pub trait CiDriver: PrivateDriver {
    /// Determines if we're on the main branch of
    /// a repository.
    fn on_default_branch() -> bool;

    /// Retrieve the certificate identity for
    /// keyless signing.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn keyless_cert_identity() -> Result<String>;

    /// Retrieve the OIDC Provider for keyless signing.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn oidc_provider() -> Result<String>;

    /// Generate a list of tags based on the OS version.
    ///
    /// ## CI
    /// The tags are generated based on the CI system that
    /// is detected. The general format for the default branch is:
    /// - `${os_version}`
    /// - `${timestamp}-${os_version}`
    ///
    /// On a branch:
    /// - `br-${branch_name}-${os_version}`
    ///
    /// In a PR(GitHub)/MR(GitLab)
    /// - `pr-${pr_event_number}-${os_version}`/`mr-${mr_iid}-${os_version}`
    ///
    /// In all above cases the short git sha is also added:
    /// - `${commit_sha}-${os_version}`
    ///
    /// When `alt_tags` are not present, the following tags are added:
    /// - `latest`
    /// - `${timestamp}`
    ///
    /// ## Locally
    /// When ran locally, only a local tag is created:
    /// - `local-${os_version}`
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn generate_tags(opts: GenerateTagsOpts) -> Result<Vec<Tag>>;

    /// Generates the image name based on CI.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn generate_image_name<'a, O>(opts: O) -> Result<Reference>
    where
        O: Borrow<GenerateImageNameOpts<'a>>,
    {
        fn inner(opts: &GenerateImageNameOpts, driver_registry: &str) -> Result<Reference> {
            let image = match (opts.registry, opts.registry_namespace, opts.tag) {
                (Some(registry), Some(registry_namespace), Some(tag)) => {
                    format!(
                        "{}/{}/{}:{}",
                        registry.trim().to_lowercase(),
                        registry_namespace.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                        tag,
                    )
                }
                (Some(registry), Some(registry_namespace), None) => {
                    format!(
                        "{}/{}/{}",
                        registry.trim().to_lowercase(),
                        registry_namespace.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                    )
                }
                (Some(registry), None, None) => {
                    format!(
                        "{}/{}",
                        registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                    )
                }
                (Some(registry), None, Some(tag)) => {
                    format!(
                        "{}/{}:{}",
                        registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                        tag,
                    )
                }
                (None, Some(namespace), None) => {
                    format!(
                        "{}/{}/{}",
                        driver_registry.trim().to_lowercase(),
                        namespace.trim().to_lowercase(),
                        opts.name.trim().to_lowercase()
                    )
                }
                (None, Some(namespace), Some(tag)) => {
                    format!(
                        "{}/{}/{}:{}",
                        driver_registry.trim().to_lowercase(),
                        namespace.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                        tag,
                    )
                }
                (None, None, Some(tag)) => {
                    format!(
                        "{}/{}:{}",
                        driver_registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                        tag,
                    )
                }
                (None, None, None) => {
                    format!(
                        "{}/{}",
                        driver_registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase(),
                    )
                }
            };
            image
                .parse()
                .into_diagnostic()
                .with_context(|| format!("Unable to parse image {image}"))
        }
        inner(opts.borrow(), &Self::get_registry()?)
    }

    /// Get the URL for the repository.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn get_repo_url() -> Result<String>;

    /// Get the registry ref for the image.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn get_registry() -> Result<String>;

    fn default_ci_file_path() -> PathBuf;
}

#[expect(private_bounds)]
pub trait BootDriver: PrivateDriver {
    /// Get the status of the current booted image.
    ///
    /// # Errors
    /// Will error if we fail to get the status.
    fn status() -> Result<Box<dyn BootStatus>>;

    /// Switch to a new image.
    ///
    /// # Errors
    /// Will error if we fail to switch to a new image.
    fn switch(opts: SwitchOpts) -> Result<()>;

    /// Upgrade an image.
    ///
    /// # Errors
    /// Will error if we fail to upgrade to a new image.
    fn upgrade(opts: SwitchOpts) -> Result<()>;
}

#[expect(private_bounds)]
pub trait BootStatus: PrivateDriver {
    /// Checks to see if there's a transaction in progress.
    fn transaction_in_progress(&self) -> bool;

    /// Gets the booted image.
    fn booted_image(&self) -> Option<ImageRef<'_>>;

    /// Gets the staged image.
    fn staged_image(&self) -> Option<ImageRef<'_>>;
}
