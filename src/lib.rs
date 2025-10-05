//! The root library for blue-build.
#![doc = include_str!("../README.md")]

use blue_build_process_management::drivers::types::BuildDriverType;
use blue_build_template::BuildEngine;

mod build_scripts;
pub mod commands;

pub use build_scripts::*;

shadow_rs::shadow!(shadow);

pub(crate) trait DriverTemplate {
    fn build_engine(&self) -> BuildEngine;
}

impl DriverTemplate for BuildDriverType {
    fn build_engine(&self) -> BuildEngine {
        match self {
            Self::Buildah | Self::Podman => BuildEngine::Oci,
            Self::Docker => BuildEngine::Docker,
        }
    }
}
