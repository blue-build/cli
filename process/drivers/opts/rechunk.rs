use std::{borrow::Cow, path::Path};

use bon::Builder;

use crate::drivers::types::{ContainerId, Platform};

use super::CompressionType;

#[derive(Debug, Clone, Builder)]
#[builder(on(Cow<'_, str>, into))]
pub struct RechunkOpts<'scope> {
    pub image: Cow<'scope, str>,

    #[builder(into)]
    pub containerfile: Cow<'scope, Path>,

    #[builder(default)]
    pub platform: Platform,
    pub version: Cow<'scope, str>,
    pub name: Cow<'scope, str>,
    pub description: Cow<'scope, str>,
    pub base_digest: Cow<'scope, str>,
    pub base_image: Cow<'scope, str>,
    pub repo: Cow<'scope, str>,

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
    pub tempdir: Option<&'scope Path>,

    #[builder(default)]
    pub clear_plan: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct ContainerOpts<'scope> {
    pub container_id: &'scope ContainerId,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct VolumeOpts<'scope> {
    #[builder(into)]
    pub volume_id: Cow<'scope, str>,

    #[builder(default)]
    pub privileged: bool,
}
