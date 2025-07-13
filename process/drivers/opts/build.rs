use std::{borrow::Cow, collections::HashSet, path::Path};

use blue_build_utils::secret::Secret;
use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::{ImageRef, Platform};

use super::CompressionType;

/// Options for building
#[derive(Debug, Clone, Builder)]
pub struct BuildOpts<'scope> {
    #[builder(into)]
    pub image: ImageRef<'scope>,

    #[builder(default)]
    pub squash: bool,

    #[builder(into)]
    pub containerfile: Cow<'scope, Path>,

    #[builder(default)]
    pub platform: Platform,

    #[builder(default)]
    pub host_network: bool,

    #[builder(default)]
    pub privileged: bool,

    #[builder(into)]
    pub cache_from: Option<&'scope Reference>,

    #[builder(into)]
    pub cache_to: Option<&'scope Reference>,

    #[builder(default)]
    pub secrets: HashSet<&'scope Secret>,
}

#[derive(Debug, Clone, Builder)]
pub struct TagOpts<'scope> {
    pub src_image: &'scope Reference,
    pub dest_image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct PushOpts<'scope> {
    pub image: &'scope Reference,
    pub compression_type: Option<CompressionType>,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct PruneOpts {
    pub all: bool,
    pub volumes: bool,
}

/// Options for building, tagging, and pusing images.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Builder)]
pub struct BuildTagPushOpts<'scope> {
    /// The base image name.
    #[builder(into)]
    pub image: ImageRef<'scope>,

    /// The path to the Containerfile to build.
    #[builder(into)]
    pub containerfile: Cow<'scope, Path>,

    /// The list of tags for the image being built.
    #[builder(default, into)]
    pub tags: Vec<Cow<'scope, str>>,

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
    #[builder(default)]
    pub platform: Platform,

    /// Runs the build with elevated privileges
    #[builder(default)]
    pub privileged: bool,

    /// Cache layers from the registry.
    pub cache_from: Option<&'scope Reference>,

    /// Cache layers to the registry.
    pub cache_to: Option<&'scope Reference>,

    /// Secrets to mount
    #[builder(default)]
    pub secrets: HashSet<&'scope Secret>,
}
