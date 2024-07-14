use std::process::{ExitStatus, Output};

use blue_build_recipe::Recipe;
use log::{debug, info, trace};
use miette::{bail, miette, Result};
use semver::{Version, VersionReq};

use crate::image_metadata::ImageMetadata;

use super::opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts};

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
                .ok_or_else(|| miette!("Image is required in order to tag"))?;
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

pub trait RunDriver: Sync + Send {
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

pub trait SigningDriver {
    /// Generate a new private/public key pair.
    ///
    /// # Errors
    /// Will error if a key-pair couldn't be generated.
    fn generate_key_pair() -> Result<()>;

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
        T: AsRef<str>;

    /// Runs the login logic for the signing driver.
    ///
    /// # Errors
    /// Will error if login fails.
    fn signing_login() -> Result<()>;
}

/// Allows agnostic retrieval of CI-based information.
pub trait CiDriver {
    /// Determines if we're on the main branch of
    /// a repository.
    fn on_default_branch() -> bool;

    /// Retrieve the certificate identity for
    /// keyless signing.
    ///
    /// # Errors
    /// Will error if the environment variables aren't set.
    fn keyless_cert_identity() -> Result<String>;

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
    fn generate_tags(recipe: &Recipe) -> Result<Vec<String>>;

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
}
