use std::borrow::Cow;

use oci_distribution::Reference;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct GenerateTagsOpts<'scope> {
    pub oci_ref: &'scope Reference,

    #[builder(default, setter(into))]
    pub alt_tags: Option<Vec<Cow<'scope, str>>>,
}
