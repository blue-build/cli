use std::borrow::Cow;

use bon::Builder;

use crate::drivers::types::Platform;

#[derive(Debug, Clone, Builder)]
#[builder(derive(Clone))]
pub struct GetMetadataOpts<'scope> {
    #[builder(into)]
    pub image: Cow<'scope, str>,

    #[builder(into)]
    pub tag: Option<Cow<'scope, str>>,

    #[builder(default)]
    pub platform: Platform,
}
