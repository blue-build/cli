use std::borrow::Cow;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,

    #[builder(default, setter(into))]
    pub args: Vec<&'scope str>,

    #[builder(default, setter(into))]
    pub env_vars: Vec<RunOptsEnv<'scope>>,

    #[builder(default, setter(into))]
    pub volumes: Vec<RunOptsVolume<'scope>>,

    #[builder(default, setter(into))]
    pub workdir: Cow<'scope, str>,

    #[builder(default)]
    pub privileged: bool,

    #[builder(default)]
    pub pull: bool,

    #[builder(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsVolume<'scope> {
    #[builder(setter(into))]
    pub path_or_vol_name: Cow<'scope, str>,

    #[builder(setter(into))]
    pub container_path: Cow<'scope, str>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsEnv<'scope> {
    #[builder(setter(into))]
    pub key: Cow<'scope, str>,

    #[builder(setter(into))]
    pub value: Cow<'scope, str>,
}
