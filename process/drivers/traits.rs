use std::{
    borrow::Borrow,
    path::PathBuf,
    process::{ExitStatus, Output},
};

use blue_build_utils::{constants::COSIGN_PUB_PATH, retry, semver::Version, string_vec};
use log::{debug, info, trace};
use miette::{bail, Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
use semver::VersionReq;

use crate::drivers::{functions::get_private_key, types::CiDriverType, Driver};

#[cfg(feature = "sigstore")]
use super::sigstore_driver::SigstoreDriver;
use super::{
    buildah_driver::BuildahDriver,
    cosign_driver::CosignDriver,
    docker_driver::DockerDriver,
    github_driver::GithubDriver,
    gitlab_driver::GitlabDriver,
    local_driver::LocalDriver,
    opts::{
        BuildOpts, BuildTagPushOpts, CheckKeyPairOpts, CreateContainerOpts, GenerateImageNameOpts,
        GenerateKeyPairOpts, GenerateTagsOpts, GetMetadataOpts, PushOpts, RemoveContainerOpts,
        RemoveImageOpts, RunOpts, SignOpts, SignVerifyOpts, TagOpts, VerifyOpts, VerifyType,
    },
    podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
    types::{ContainerId, ImageMetadata},
};
#[cfg(feature = "rechunk")]
use super::{opts::RechunkOpts, types::MountId};

trait PrivateDriver {}

macro_rules! impl_private_driver {
    ($($driver:ty),* $(,)?) => {
        $(
            impl PrivateDriver for $driver {}
        )*
    };
}

impl_private_driver!(
    Driver,
    DockerDriver,
    PodmanDriver,
    BuildahDriver,
    GithubDriver,
    GitlabDriver,
    LocalDriver,
    CosignDriver,
    SkopeoDriver,
    CiDriverType,
);

#[cfg(feature = "sigstore")]
impl_private_driver!(SigstoreDriver);

/// Trait for retrieving version of a driver.
#[allow(private_bounds)]
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

/// Allows agnostic building, tagging
/// pushing, and login.
#[allow(private_bounds)]
pub trait BuildDriver: PrivateDriver {
    /// Runs the build logic for the driver.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn build(opts: &BuildOpts) -> Result<()>;

    /// Runs the tag logic for the driver.
    ///
    /// # Errors
    /// Will error if the tagging fails.
    fn tag(opts: &TagOpts) -> Result<()>;

    /// Runs the push logic for the driver
    ///
    /// # Errors
    /// Will error if the push fails.
    fn push(opts: &PushOpts) -> Result<()>;

    /// Runs the login logic for the driver.
    ///
    /// # Errors
    /// Will error if login fails.
    fn login() -> Result<()>;

    /// Runs prune commands for the driver.
    ///
    /// # Errors
    /// Will error if the driver fails to prune.
    #[cfg(feature = "prune")]
    fn prune(opts: &super::opts::PruneOpts) -> Result<()>;

    /// Runs the logic for building, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, tagging, or pusing fails.
    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<Vec<String>> {
        trace!("BuildDriver::build_tag_push({opts:#?})");

        let full_image = match (opts.archive_path.as_ref(), opts.image.as_ref()) {
            (Some(archive_path), None) => {
                format!("oci-archive:{}", archive_path.display())
            }
            (None, Some(image)) => image.to_string(),
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        };

        let build_opts = BuildOpts::builder()
            .image(&full_image)
            .containerfile(opts.containerfile.as_ref())
            .platform(opts.platform)
            .squash(opts.squash)
            .build();

        info!("Building image {full_image}");
        Self::build(&build_opts)?;

        let image_list: Vec<String> = if !opts.tags.is_empty() && opts.archive_path.is_none() {
            let image = opts.image.unwrap();
            debug!("Tagging all images");

            let mut image_list = Vec::with_capacity(opts.tags.len());

            for tag in &opts.tags {
                debug!("Tagging {} with {tag}", &full_image);
                let tagged_image: Reference =
                    format!("{}/{}:{tag}", image.resolve_registry(), image.repository())
                        .parse()
                        .into_diagnostic()?;

                let tag_opts = TagOpts::builder()
                    .src_image(image)
                    .dest_image(&tagged_image)
                    .build();

                Self::tag(&tag_opts)?;
                image_list.push(tagged_image.to_string());

                if opts.push {
                    let retry_count = if opts.retry_push { opts.retry_count } else { 0 };

                    debug!("Pushing all images");
                    // Push images with retries (1s delay between retries)
                    blue_build_utils::retry(retry_count, 5, || {
                        debug!("Pushing image {tagged_image}");

                        let push_opts = PushOpts::builder()
                            .image(&tagged_image)
                            .compression_type(opts.compression)
                            .build();

                        Self::push(&push_opts)
                    })?;
                }
            }

            image_list
        } else {
            string_vec![&full_image]
        };

        Ok(image_list)
    }
}

/// Allows agnostic inspection of images.
#[allow(private_bounds)]
pub trait InspectDriver: PrivateDriver {
    /// Gets the metadata on an image tag.
    ///
    /// # Errors
    /// Will error if it is unable to get the labels.
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata>;
}

/// Allows agnostic running of containers.
#[allow(private_bounds)]
pub trait RunDriver: PrivateDriver {
    /// Run a container to perform an action.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run(opts: &RunOpts) -> Result<ExitStatus>;

    /// Run a container to perform an action and capturing output.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run_output(opts: &RunOpts) -> Result<Output>;

    /// Creates container
    ///
    /// # Errors
    /// Will error if the container create command fails.
    fn create_container(opts: &CreateContainerOpts) -> Result<ContainerId>;

    /// Removes a container
    ///
    /// # Errors
    /// Will error if the container remove command fails.
    fn remove_container(opts: &RemoveContainerOpts) -> Result<()>;

    /// Removes an image
    ///
    /// # Errors
    /// Will error if the image remove command fails.
    fn remove_image(opts: &RemoveImageOpts) -> Result<()>;

    /// List all images in the local image registry.
    ///
    /// # Errors
    /// Will error if the image list command fails.
    fn list_images(privileged: bool) -> Result<Vec<Reference>>;
}

#[allow(private_bounds)]
#[cfg(feature = "rechunk")]
pub(super) trait ContainerMountDriver: PrivateDriver {
    /// Mounts the container
    ///
    /// # Errors
    /// Will error if the container mount command fails.
    fn mount_container(opts: &super::opts::ContainerOpts) -> Result<MountId>;

    /// Unmount the container
    ///
    /// # Errors
    /// Will error if the container unmount command fails.
    fn unmount_container(opts: &super::opts::ContainerOpts) -> Result<()>;

    /// Remove a volume
    ///
    /// # Errors
    /// Will error if the volume remove command fails.
    fn remove_volume(opts: &super::opts::VolumeOpts) -> Result<()>;
}

#[cfg(feature = "rechunk")]
pub(super) trait OciCopy {
    fn copy_oci_dir(opts: &super::opts::CopyOciDirOpts) -> Result<()>;
}

#[allow(private_bounds)]
#[cfg(feature = "rechunk")]
pub trait RechunkDriver: RunDriver + BuildDriver + ContainerMountDriver {
    const RECHUNK_IMAGE: &str = "ghcr.io/hhd-dev/rechunk:v1.0.1";

    /// Perform a rechunk build of a recipe.
    ///
    /// # Errors
    /// Will error if the rechunk process fails.
    fn rechunk(opts: &RechunkOpts) -> Result<Vec<String>> {
        let ostree_cache_id = &uuid::Uuid::new_v4().to_string();
        let raw_image =
            &Reference::try_from(format!("localhost/{ostree_cache_id}/raw-rechunk")).unwrap();
        let current_dir = &std::env::current_dir().into_diagnostic()?;
        let current_dir = &*current_dir.to_string_lossy();
        let full_image = Reference::try_from(opts.tags.first().map_or_else(
            || opts.image.to_string(),
            |tag| format!("{}:{tag}", opts.image),
        ))
        .into_diagnostic()?;

        Self::login()?;

        Self::build(
            &BuildOpts::builder()
                .image(raw_image.to_string())
                .containerfile(&*opts.containerfile)
                .platform(opts.platform)
                .privileged(true)
                .squash(true)
                .host_network(true)
                .build(),
        )?;

        let container = &Self::create_container(
            &CreateContainerOpts::builder()
                .image(raw_image)
                .privileged(true)
                .build(),
        )?;
        let mount = &Self::mount_container(
            &super::opts::ContainerOpts::builder()
                .container_id(container)
                .privileged(true)
                .build(),
        )?;

        Self::prune_image(mount, container, raw_image, opts)?;
        Self::create_ostree_commit(mount, ostree_cache_id, container, raw_image, opts)?;

        let temp_dir = if let Some(dir) = opts.tempdir {
            tempfile::TempDir::new_in(dir).into_diagnostic()?
        } else {
            tempfile::TempDir::new().into_diagnostic()?
        };
        let temp_dir_str = &*temp_dir.path().to_string_lossy();

        Self::rechunk_image(ostree_cache_id, temp_dir_str, current_dir, opts)?;

        let mut image_list = Vec::with_capacity(opts.tags.len());

        if opts.push {
            let oci_dir = &super::types::OciDir::try_from(temp_dir.path().join(ostree_cache_id))?;

            for tag in &opts.tags {
                let tagged_image = Reference::with_tag(
                    full_image.registry().to_string(),
                    full_image.repository().to_string(),
                    tag.to_string(),
                );

                blue_build_utils::retry(opts.retry_count, 5, || {
                    debug!("Pushing image {tagged_image}");

                    Driver::copy_oci_dir(
                        &super::opts::CopyOciDirOpts::builder()
                            .oci_dir(oci_dir)
                            .registry(&tagged_image)
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
        raw_image: &Reference,
        opts: &RechunkOpts<'_>,
    ) -> Result<(), miette::Error> {
        let status = Self::run(
            &RunOpts::builder()
                .image(Self::RECHUNK_IMAGE)
                .remove(true)
                .user("0:0")
                .privileged(true)
                .volumes(crate::run_volumes! {
                    mount => "/var/tree",
                })
                .env_vars(crate::run_envs! {
                    "TREE" => "/var/tree",
                })
                .args(bon::vec!["/sources/rechunk/1_prune.sh"])
                .build(),
        )?;

        if !status.success() {
            Self::unmount_container(
                &super::opts::ContainerOpts::builder()
                    .container_id(container)
                    .privileged(true)
                    .build(),
            )?;
            Self::remove_container(
                &RemoveContainerOpts::builder()
                    .container_id(container)
                    .privileged(true)
                    .build(),
            )?;
            Self::remove_image(
                &RemoveImageOpts::builder()
                    .image(raw_image)
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
        raw_image: &Reference,
        opts: &RechunkOpts<'_>,
    ) -> Result<()> {
        let status = Self::run(
            &RunOpts::builder()
                .image(Self::RECHUNK_IMAGE)
                .remove(true)
                .user("0:0")
                .privileged(true)
                .volumes(crate::run_volumes! {
                    mount => "/var/tree",
                    ostree_cache_id => "/var/ostree",
                })
                .env_vars(crate::run_envs! {
                    "TREE" => "/var/tree",
                    "REPO" => "/var/ostree/repo",
                    "RESET_TIMESTAMP" => "1",
                })
                .args(bon::vec!["/sources/rechunk/2_create.sh"])
                .build(),
        )?;
        Self::unmount_container(
            &super::opts::ContainerOpts::builder()
                .container_id(container)
                .privileged(true)
                .build(),
        )?;
        Self::remove_container(
            &RemoveContainerOpts::builder()
                .container_id(container)
                .privileged(true)
                .build(),
        )?;
        Self::remove_image(
            &RemoveImageOpts::builder()
                .image(raw_image)
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
        opts: &RechunkOpts<'_>,
    ) -> Result<()> {
        let status = Self::run(
        &RunOpts::builder()
            .image(Self::RECHUNK_IMAGE)
            .remove(true)
            .user("0:0")
            .privileged(true)
            .volumes(crate::run_volumes! {
                ostree_cache_id => "/var/ostree",
                temp_dir_str => "/workspace",
                current_dir => "/var/git"
            })
            .env_vars(crate::run_envs! {
                "REPO" => "/var/ostree/repo",
                "PREV_REF" => &*opts.image,
                "OUT_NAME" => ostree_cache_id,
                "CLEAR_PLAN" => if opts.clear_plan { "true" } else { "" },
                "VERSION" => format!("{}", opts.version),
                "OUT_REF" => format!("oci:{ostree_cache_id}"),
                "GIT_DIR" => "/var/git",
                "LABELS" => format!(
                    "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                    format_args!("{}={}", blue_build_utils::constants::BUILD_ID_LABEL, Driver::get_build_id()),
                    format_args!("org.opencontainers.image.title={}", &opts.name),
                    format_args!("org.opencontainers.image.description={}", &opts.description),
                    format_args!("org.opencontainers.image.source={}", &opts.repo),
                    format_args!("org.opencontainers.image.base.digest={}", &opts.base_digest),
                    format_args!("org.opencontainers.image.base.name={}", &opts.base_image),
                    "org.opencontainers.image.created=<timestamp>",
                    "io.artifacthub.package.readme-url=https://raw.githubusercontent.com/blue-build/cli/main/README.md",
                )
            })
            .args(bon::vec!["/sources/rechunk/3_chunk.sh"])
            .build(),
        )?;

        Self::remove_volume(
            &super::opts::VolumeOpts::builder()
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
#[allow(private_bounds)]
pub trait SigningDriver: PrivateDriver {
    /// Generate a new private/public key pair.
    ///
    /// # Errors
    /// Will error if a key-pair couldn't be generated.
    fn generate_key_pair(opts: &GenerateKeyPairOpts) -> Result<()>;

    /// Checks the signing key files to ensure
    /// they match.
    ///
    /// # Errors
    /// Will error if the files cannot be verified.
    fn check_signing_files(opts: &CheckKeyPairOpts) -> Result<()>;

    /// Signs the image digest.
    ///
    /// # Errors
    /// Will error if signing fails.
    fn sign(opts: &SignOpts) -> Result<()>;

    /// Verifies the image.
    ///
    /// The image can be verified either with `VerifyType::File` containing
    /// the public key contents, or with `VerifyType::Keyless` containing
    /// information about the `issuer` and `identity`.
    ///
    /// # Errors
    /// Will error if the image fails to be verified.
    fn verify(opts: &VerifyOpts) -> Result<()>;

    /// Sign an image given the image name and tag.
    ///
    /// # Errors
    /// Will error if the image fails to be signed.
    fn sign_and_verify(opts: &SignVerifyOpts) -> Result<()> {
        trace!("sign_and_verify({opts:?})");

        let path = opts
            .dir
            .as_ref()
            .map_or_else(|| PathBuf::from("."), |d| d.to_path_buf());

        let image_digest = Driver::get_metadata(
            &GetMetadataOpts::builder()
                .image(opts.image)
                .platform(opts.platform)
                .build(),
        )?
        .digest;
        let image_digest: Reference = format!(
            "{}/{}@{image_digest}",
            opts.image.resolve_registry(),
            opts.image.repository(),
        )
        .parse()
        .into_diagnostic()?;

        let (sign_opts, verify_opts) = match (Driver::get_ci_driver(), get_private_key(&path)) {
            // Cosign public/private key pair
            (_, Ok(priv_key)) => (
                SignOpts::builder()
                    .image(&image_digest)
                    .dir(&path)
                    .key(priv_key.to_string())
                    .build(),
                VerifyOpts::builder()
                    .image(opts.image)
                    .verify_type(VerifyType::File(path.join(COSIGN_PUB_PATH).into()))
                    .build(),
            ),
            // Gitlab keyless
            (CiDriverType::Github | CiDriverType::Gitlab, _) => (
                SignOpts::builder().dir(&path).image(&image_digest).build(),
                VerifyOpts::builder()
                    .image(opts.image)
                    .verify_type(VerifyType::Keyless {
                        issuer: Driver::oidc_provider()?.into(),
                        identity: Driver::keyless_cert_identity()?.into(),
                    })
                    .build(),
            ),
            _ => bail!("Failed to get information for signing the image"),
        };

        let retry_count = if opts.retry_push { opts.retry_count } else { 0 };

        retry(retry_count, 5, || {
            Self::sign(&sign_opts)?;
            Self::verify(&verify_opts)
        })?;

        Ok(())
    }

    /// Runs the login logic for the signing driver.
    ///
    /// # Errors
    /// Will error if login fails.
    fn signing_login() -> Result<()>;
}

/// Allows agnostic retrieval of CI-based information.
#[allow(private_bounds)]
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
    fn generate_tags(opts: &GenerateTagsOpts) -> Result<Vec<String>>;

    /// Generates the image name based on CI.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn generate_image_name<'a, O>(opts: O) -> Result<Reference>
    where
        O: Borrow<GenerateImageNameOpts<'a>>,
    {
        fn inner(opts: &GenerateImageNameOpts, driver_registry: &str) -> Result<Reference> {
            let image = match (&opts.registry, &opts.registry_namespace) {
                (Some(registry), Some(registry_namespace)) => {
                    format!(
                        "{}/{}/{}",
                        registry.trim().to_lowercase(),
                        registry_namespace.trim().to_lowercase(),
                        opts.name.trim().to_lowercase()
                    )
                }
                (Some(registry), None) => {
                    format!(
                        "{}/{}",
                        registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase()
                    )
                }
                _ => {
                    format!(
                        "{}/{}",
                        driver_registry.trim().to_lowercase(),
                        opts.name.trim().to_lowercase()
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
