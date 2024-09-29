use std::borrow::Cow;

use bon::Builder;

#[derive(Debug, Clone, Builder)]
pub struct GetMetadataOpts<'scope> {
    #[builder(into)]
    pub image: Cow<'scope, str>,

    #[builder(into)]
    pub tag: Option<Cow<'scope, str>>,
}
