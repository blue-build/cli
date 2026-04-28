use blue_build_utils::{container::Tag, platform::Platform, tagging::TaggingPolicy};
use bon::Builder;
use oci_client::Reference;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct GenerateTagsOpts<'scope> {
    pub oci_ref: &'scope Reference,

    #[builder(into)]
    pub alt_tags: Option<&'scope [Tag]>,

    #[builder(into)]
    pub tags: Option<&'scope [String]>,

    #[builder(into)]
    pub tagging: Option<&'scope [TaggingPolicy]>,

    pub os_version: &'scope str,
    pub timestamp: &'scope str,
    pub short_sha: Option<&'scope str>,

    pub platform: Option<Platform>,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct GenerateImageNameOpts<'scope> {
    pub name: &'scope str,
    pub registry: Option<&'scope str>,
    pub registry_namespace: Option<&'scope str>,
    pub tag: Option<&'scope Tag>,
}
