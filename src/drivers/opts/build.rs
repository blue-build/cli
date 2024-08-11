use std::{borrow::Cow, path::Path};

use typed_builder::TypedBuilder;

use super::CompressionType;

/// Options for building
#[derive(Debug, Clone, TypedBuilder)]
pub struct BuildOpts<'a> {
    #[builder(setter(into))]
    pub image: Cow<'a, str>,

    #[builder(default)]
    pub squash: bool,

    #[builder(setter(into))]
    pub containerfile: Cow<'a, Path>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct TagOpts<'a> {
    #[builder(setter(into))]
    pub src_image: Cow<'a, str>,

    #[builder(setter(into))]
    pub dest_image: Cow<'a, str>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct PushOpts<'a> {
    #[builder(setter(into))]
    pub image: Cow<'a, str>,

    #[builder(default, setter(strip_option))]
    pub compression_type: Option<CompressionType>,
}

/// Options for building, tagging, and pusing images.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, TypedBuilder)]
pub struct BuildTagPushOpts<'a> {
    /// The base image name.
    ///
    /// NOTE: You cannot have this set with `archive_path` set.
    #[builder(default, setter(into, strip_option))]
    pub image: Option<Cow<'a, str>>,

    /// The path to the archive file.
    ///
    /// NOTE: You cannot have this set with image set.
    #[builder(default, setter(into, strip_option))]
    pub archive_path: Option<Cow<'a, str>>,

    /// The path to the Containerfile to build.
    #[builder(setter(into))]
    pub containerfile: Cow<'a, Path>,

    /// The list of tags for the image being built.
    #[builder(default, setter(into))]
    pub tags: Cow<'a, [&'a str]>,

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
}
