use std::borrow::Cow;

use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,

    #[builder(default, setter(into))]
    pub args: Cow<'scope, [String]>,

    #[builder(default, setter(into))]
    pub env_vars: Cow<'scope, [RunOptsEnv<'scope>]>,

    #[builder(default, setter(into))]
    pub volumes: Cow<'scope, [RunOptsVolume<'scope>]>,

    #[builder(default, setter(strip_option))]
    pub uid: Option<u32>,

    #[builder(default, setter(strip_option))]
    pub gid: Option<u32>,

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

#[macro_export]
macro_rules! run_volumes {
    ($($host:expr => $container:expr),+ $(,)?) => {
        {
            [
                $($crate::drivers::opts::RunOptsVolume::builder()
                    .path_or_vol_name($host)
                    .container_path($container)
                    .build(),)*
            ]
        }
    };
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct RunOptsEnv<'scope> {
    #[builder(setter(into))]
    pub key: Cow<'scope, str>,

    #[builder(setter(into))]
    pub value: Cow<'scope, str>,
}

#[macro_export]
macro_rules! run_envs {
    ($($key:expr => $value:expr),+ $(,)?) => {
        {
            [
                $($crate::drivers::opts::RunOptsEnv::builder()
                    .key($key)
                    .value($value)
                    .build(),)*
            ]
        }
    };
}
