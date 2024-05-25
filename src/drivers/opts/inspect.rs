use std::borrow::Cow;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct GetMetadataOpts<'a> {
    #[builder(setter(into))]
    pub image: Cow<'a, str>,

    #[builder(default, setter(into, strip_option))]
    pub tag: Option<Cow<'a, str>>,
}
