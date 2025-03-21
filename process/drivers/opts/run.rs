use std::borrow::Cow;

use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::ContainerId;

#[derive(Debug, Clone, Builder)]
pub struct RunOpts<'scope> {
    #[builder(into)]
    pub image: Cow<'scope, str>,

    #[builder(default, into)]
    pub args: Vec<Cow<'scope, str>>,

    #[builder(default, into)]
    pub env_vars: Vec<RunOptsEnv<'scope>>,

    #[builder(default, into)]
    pub volumes: Vec<RunOptsVolume<'scope>>,

    #[builder(into)]
    pub user: Option<Cow<'scope, str>>,

    #[builder(default)]
    pub privileged: bool,

    #[builder(default)]
    pub pull: bool,

    #[builder(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct RunOptsVolume<'scope> {
    #[builder(into)]
    pub path_or_vol_name: Cow<'scope, str>,

    #[builder(into)]
    pub container_path: Cow<'scope, str>,
}

#[macro_export]
macro_rules! run_volumes {
    ($($host:expr => $container:expr),+ $(,)?) => {
        {
            ::bon::vec![
                $($crate::drivers::opts::RunOptsVolume::builder()
                    .path_or_vol_name($host)
                    .container_path($container)
                    .build(),)*
            ]
        }
    };
}

#[derive(Debug, Clone, Builder)]
pub struct RunOptsEnv<'scope> {
    #[builder(into)]
    pub key: Cow<'scope, str>,

    #[builder(into)]
    pub value: Cow<'scope, str>,
}

#[macro_export]
macro_rules! run_envs {
    ($($key:expr => $value:expr),+ $(,)?) => {
        {
            ::bon::vec![
                $($crate::drivers::opts::RunOptsEnv::builder()
                    .key($key)
                    .value($value)
                    .build(),)*
            ]
        }
    };
}

#[derive(Debug, Clone, Builder)]
pub struct CreateContainerOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct RemoveContainerOpts<'scope> {
    pub container_id: &'scope ContainerId,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Builder)]
pub struct RemoveImageOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}
