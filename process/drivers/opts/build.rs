use std::{borrow::Cow, path::Path};

use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::Platform;

use super::CompressionType;

/// Options for building
#[derive(Debug, Clone, Builder)]
pub struct BuildOpts<'scope> {
    #[builder(into)]
    pub image: Cow<'scope, str>,

    #[builder(default)]
    pub squash: bool,

    #[builder(into)]
    pub containerfile: Cow<'scope, Path>,

    #[builder(default)]
    pub platform: Platform,

    #[builder(default)]
    pub host_network: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct TagOpts<'scope> {
    pub src_image: &'scope Reference,
    pub dest_image: &'scope Reference,
}

#[derive(Debug, Clone, Builder)]
pub struct PushOpts<'scope> {
    pub image: &'scope Reference,
    pub compression_type: Option<CompressionType>,
}

#[derive(Debug, Clone, Builder)]
#[cfg(feature = "prune")]
pub struct PruneOpts {
    pub all: bool,
    pub volumes: bool,
}

/// Options for building, tagging, and pusing images.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Builder)]
pub struct BuildTagPushOpts<'scope> {
    /// The base image name.
    ///
    /// NOTE: This SHOULD NOT contain the tag of the image.
    ///
    /// NOTE: You cannot have this set with `archive_path` set.
    pub image: Option<&'scope Reference>,

    /// The path to the archive file.
    ///
    /// NOTE: You cannot have this set with image set.
    #[builder(into)]
    pub archive_path: Option<Cow<'scope, Path>>,

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
}
