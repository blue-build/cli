use std::{
    borrow::Borrow,
    path::PathBuf,
    process::{ExitStatus, Output},
};

use blue_build_utils::{constants::COSIGN_PUB_PATH, retry, string_vec};
use log::{debug, info, trace};
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
use semver::{Version, VersionReq};

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
        BuildOpts, BuildTagPushOpts, CheckKeyPairOpts, GenerateImageNameOpts, GenerateKeyPairOpts,
        GenerateTagsOpts, GetMetadataOpts, PushOpts, RunOpts, SignOpts, SignVerifyOpts, TagOpts,
        VerifyOpts, VerifyType,
    },
    podman_driver::PodmanDriver,
    skopeo_driver::SkopeoDriver,
    types::ImageMetadata,
};

trait PrivateDriver {}

macro_rules! impl_private_driver {
    ($driver:ty) => {
        impl PrivateDriver for $driver {}
    };
}

impl_private_driver!(Driver);
impl_private_driver!(DockerDriver);
impl_private_driver!(PodmanDriver);
impl_private_driver!(BuildahDriver);
impl_private_driver!(GithubDriver);
impl_private_driver!(GitlabDriver);
impl_private_driver!(LocalDriver);
impl_private_driver!(CosignDriver);
impl_private_driver!(SkopeoDriver);

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

    /// Runs the logic for building, tagging, and pushing an image.
    ///
    /// # Errors
    /// Will error if building, tagging, or pusing fails.
    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<Vec<String>> {
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

        let image_list: Vec<String> = if !opts.tags.is_empty() && opts.archive_path.is_none() {
            let image = opts
                .image
                .as_ref()
                .ok_or_else(|| miette!("Image is required in order to tag"))?;
            debug!("Tagging all images");

            let mut image_list = Vec::with_capacity(opts.tags.len());

            for tag in &opts.tags {
                debug!("Tagging {} with {tag}", &full_image);
                let tagged_image = format!("{image}:{tag}");

                let tag_opts = TagOpts::builder()
                    .src_image(&full_image)
                    .dest_image(&tagged_image)
                    .build();

                Self::tag(&tag_opts)?;
                image_list.push(tagged_image);

                if opts.push {
                    let retry_count = if opts.retry_push { opts.retry_count } else { 0 };

                    debug!("Pushing all images");
                    // Push images with retries (1s delay between retries)
                    blue_build_utils::retry(retry_count, 5, || {
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
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus>;

    /// Run a container to perform an action and capturing output.
    ///
    /// # Errors
    /// Will error if there is an issue running the container.
    fn run_output(opts: &RunOpts) -> std::io::Result<Output>;
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

        let image_name: &str = opts.image.as_ref();
        let inspect_opts = GetMetadataOpts::builder().image(image_name);

        let inspect_opts = if let Some(ref tag) = opts.tag {
            inspect_opts.tag(tag.as_ref() as &str).build()
        } else {
            inspect_opts.build()
        };

        let image_digest = Driver::get_metadata(&inspect_opts)?.digest;
        let image_name_tag = opts
            .tag
            .as_ref()
            .map_or_else(|| image_name.to_owned(), |t| format!("{image_name}:{t}"));
        let image_digest = format!("{image_name}@{image_digest}");

        let (sign_opts, verify_opts) = match (Driver::get_ci_driver(), get_private_key(&path)) {
            // Cosign public/private key pair
            (_, Ok(priv_key)) => (
                SignOpts::builder()
                    .image(&image_digest)
                    .dir(&path)
                    .key(priv_key.to_string())
                    .build(),
                VerifyOpts::builder()
                    .image(&image_name_tag)
                    .verify_type(VerifyType::File(path.join(COSIGN_PUB_PATH).into()))
                    .build(),
            ),
            // Gitlab keyless
            (CiDriverType::Github | CiDriverType::Gitlab, _) => (
                SignOpts::builder().dir(&path).image(&image_digest).build(),
                VerifyOpts::builder()
                    .image(&image_name_tag)
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
