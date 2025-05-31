use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::ContainerId;

#[derive(Debug, Clone, Copy, Builder)]
pub struct RunOpts<'scope> {
    pub image: &'scope str,

    #[builder(default)]
    pub args: &'scope [String],

    #[builder(default)]
    pub env_vars: &'scope [RunOptsEnv<'scope>],

    #[builder(default)]
    pub volumes: &'scope [RunOptsVolume<'scope>],
    pub user: Option<&'scope str>,

    #[builder(default)]
    pub privileged: bool,

    #[builder(default)]
    pub pull: bool,

    #[builder(default)]
    pub remove: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct RunOptsVolume<'scope> {
    pub path_or_vol_name: &'scope str,
    pub container_path: &'scope str,
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

#[derive(Debug, Clone, Copy, Builder)]
pub struct RunOptsEnv<'scope> {
    pub key: &'scope str,
    pub value: &'scope str,
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

#[derive(Debug, Clone, Copy, Builder)]
pub struct CreateContainerOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct RemoveContainerOpts<'scope> {
    pub container_id: &'scope ContainerId,

    #[builder(default)]
    pub privileged: bool,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct RemoveImageOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}
