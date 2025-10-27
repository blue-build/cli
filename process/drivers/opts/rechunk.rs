use std::{collections::BTreeMap, path::Path};

use blue_build_utils::{
    container::{ContainerId, OciDir, Tag},
    platform::Platform,
    secret::Secret,
};
use bon::Builder;
use oci_distribution::Reference;

use super::CompressionType;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct RechunkOpts<'scope> {
    pub image: &'scope Reference,
    pub containerfile: &'scope Path,
    pub labels: &'scope BTreeMap<String, String>,

    pub platform: &'scope [Platform],
    pub version: &'scope str,
    pub name: &'scope str,
    pub description: &'scope str,
    pub base_digest: &'scope str,
    pub base_image: &'scope Reference,
    pub repo: &'scope str,

    /// The list of tags for the image being built.
    #[builder(default)]
    pub tags: &'scope [Tag],

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

    /// Cache layers from the registry.
    pub cache_from: Option<&'scope Reference>,

    /// Cache layers to the registry.
    pub cache_to: Option<&'scope Reference>,

    #[builder(default)]
    pub secrets: &'scope [&'scope Secret],
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct ContainerOpts<'scope> {
    pub container_id: &'scope ContainerId,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct VolumeOpts<'scope> {
    pub volume_id: &'scope str,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct CopyOciDirOpts<'scope> {
    pub oci_dir: &'scope OciDir,
    pub registry: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}
