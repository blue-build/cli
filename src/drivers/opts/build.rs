use std::borrow::Cow;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct BuildTagPushOpts<'a> {
    #[builder(default, setter(into, strip_option))]
    pub image: Option<Cow<'a, str>>,

    #[builder(default, setter(into, strip_option))]
    pub archive_path: Option<Cow<'a, str>>,

    #[builder(default, setter(into))]
    pub tags: Cow<'a, [&'a str]>,

    #[builder(default)]
    pub push: bool,

    #[builder(default)]
    pub no_retry_push: bool,

    #[builder(default = 1)]
    pub retry_count: u8,
}
