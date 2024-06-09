use std::{borrow::Cow, path::Path};

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOpts<'a> {
    #[builder(default, setter(into))]
    pub args: Cow<'a, [Cow<'a, str>]>,

    #[builder(default, setter(into))]
    pub env_vars: Cow<'a, [RunOptsEnv<'a>]>,

    #[builder(default, setter(into))]
    pub volumes: Cow<'a, [RunOptsVolumes<'a>]>,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsVolumes<'a> {
    #[builder(setter(into))]
    pub host_path: Cow<'a, Path>,

    #[builder(setter(into))]
    pub container_path: Cow<'a, Path>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsEnv<'a> {
    pub key: Cow<'a, str>,
    pub value: Cow<'a, str>,
}
