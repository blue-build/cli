use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::Platform;

#[derive(Debug, Clone, Copy, Builder)]
pub struct GenerateTagsOpts<'scope> {
    pub oci_ref: &'scope Reference,

    #[builder(into)]
    pub alt_tags: Option<&'scope [String]>,

    #[builder(default)]
    pub platform: Platform,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct GenerateImageNameOpts<'scope> {
    pub name: &'scope str,
    pub registry: Option<&'scope str>,
    pub registry_namespace: Option<&'scope str>,
}
