use blue_build_utils::{container::Tag, platform::Platform};
use bon::Builder;
use oci_distribution::Reference;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct GenerateTagsOpts<'scope> {
    pub oci_ref: &'scope Reference,

    #[builder(into)]
    pub alt_tags: Option<&'scope [Tag]>,

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
