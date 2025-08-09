use std::path::Path;

use blue_build_utils::secret::Secret;
use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::{ImageRef, Platform};

use super::CompressionType;

/// Options for building
#[derive(Debug, Clone, Copy, Builder)]
pub struct BuildOpts<'scope> {
    pub image: &'scope ImageRef<'scope>,

    #[builder(default)]
    pub squash: bool,

    pub containerfile: &'scope Path,

    pub platform: Option<Platform>,

    #[builder(default)]
    pub host_network: bool,

    #[builder(default)]
    pub privileged: bool,
    pub cache_from: Option<&'scope Reference>,
    pub cache_to: Option<&'scope Reference>,

    #[builder(default)]
    pub secrets: &'scope [&'scope Secret],
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct TagOpts<'scope> {
    pub src_image: &'scope Reference,
    pub dest_image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct PushOpts<'scope> {
    pub image: &'scope Reference,
    pub compression_type: Option<CompressionType>,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct PruneOpts {
    pub all: bool,
    pub volumes: bool,
}

/// Options for building, tagging, and pusing images.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Builder)]
pub struct BuildTagPushOpts<'scope> {
    /// The base image name.
    pub image: &'scope ImageRef<'scope>,

    /// The path to the Containerfile to build.
    pub containerfile: &'scope Path,

    /// The list of tags for the image being built.
    #[builder(default)]
    pub tags: &'scope [String],

    /// Enable pushing the image.
    #[builder(default)]
    pub push: bool,

    /// Enable retry logic for pushing.
    #[builder(default)]
    pub retry_push: bool,

    /// Number of times to retry pushing.
    ///
    /// Defaults to 1.
    #[builder(default = 1)]
    pub retry_count: u8,

    /// The compression type to use when pushing.
    #[builder(default)]
    pub compression: CompressionType,

    /// Run all steps in a single layer.
    #[builder(default)]
    pub squash: bool,

    /// The platform to build the image on.
    pub platform: Option<Platform>,

    /// Runs the build with elevated privileges
    #[builder(default)]
    pub privileged: bool,

    /// Cache layers from the registry.
    pub cache_from: Option<&'scope Reference>,

    /// Cache layers to the registry.
    pub cache_to: Option<&'scope Reference>,

    /// Secrets to mount
    #[builder(default)]
    pub secrets: &'scope [&'scope Secret],
}
