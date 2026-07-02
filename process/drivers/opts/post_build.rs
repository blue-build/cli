use blue_build_utils::{container::ImageRef, platform::Platform};
use bon::Builder;

use crate::drivers::PostBuild;

/// Options passed to the post-build hook for each image to be processed
#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct PostBuildOpts<'scope> {
    /// The image reference to be postprocessed.
    pub input_image: &'scope ImageRef<'scope>,

    /// The image reference where the postprocessed image should be placed.
    pub output_image: &'scope ImageRef<'scope>,

    /// The image reference of a previous build that may be taken into account.
    pub previous_image: Option<&'scope ImageRef<'scope>>,

    /// The platform of the image.
    pub platform: Platform,

    /// Runs post-processing with elevated privileges.
    #[builder(default)]
    pub privileged: bool,
}

/// Options for the post-build driver
#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct PostBuildDriverOpts<'scope> {
    /// Post-build hook (e.g. for rechunking)
    pub post_build: &'scope dyn PostBuild,

    /// Whether to remove the base image after building
    pub remove_base_image: bool,

    /// Whether to take a previous image into account.
    #[builder(default)]
    pub use_previous_image: bool,
}
