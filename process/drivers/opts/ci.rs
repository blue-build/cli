use std::borrow::Cow;

use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::Platform;

#[derive(Debug, Clone, Builder)]
pub struct GenerateTagsOpts<'scope> {
    pub oci_ref: &'scope Reference,

    #[builder(into)]
    pub alt_tags: Option<Vec<Cow<'scope, str>>>,

    #[builder(default)]
    pub platform: Platform,
}

#[derive(Debug, Clone, Builder)]
pub struct GenerateImageNameOpts<'scope> {
    #[builder(into)]
    pub name: Cow<'scope, str>,

    #[builder(into)]
    pub registry: Option<Cow<'scope, str>>,

    #[builder(into)]
    pub registry_namespace: Option<Cow<'scope, str>>,
}
