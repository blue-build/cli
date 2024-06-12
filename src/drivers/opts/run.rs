use std::borrow::Cow;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOpts<'a> {
    #[builder(default, setter(into))]
    pub image: Cow<'a, str>,

    #[builder(default, setter(into))]
    pub args: Cow<'a, [String]>,

    #[builder(default, setter(into))]
    pub env_vars: Cow<'a, [RunOptsEnv<'a>]>,

    #[builder(default, setter(into))]
    pub volumes: Cow<'a, [RunOptsVolume<'a>]>,

    #[builder(default)]
    pub privileged: bool,

    #[builder(default)]
    pub pull: bool,

    #[builder(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsVolume<'a> {
    #[builder(setter(into))]
    pub path_or_vol_name: Cow<'a, str>,

    #[builder(setter(into))]
    pub container_path: Cow<'a, str>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsEnv<'a> {
    #[builder(setter(into))]
    pub key: Cow<'a, str>,

    #[builder(setter(into))]
    pub value: Cow<'a, str>,
}
