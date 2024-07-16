use std::env;

use blue_build_utils::constants::{GITHUB_ACTIONS, GITLAB_CI};
use clap::ValueEnum;
use log::trace;

use crate::drivers::{
    buildah_driver::BuildahDriver, docker_driver::DockerDriver, podman_driver::PodmanDriver,
    DriverVersion,
};

pub(super) trait DetermineDriver<T> {
    fn determine_driver(&mut self) -> T;
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InspectDriverType {
    Skopeo,
    Podman,
    Docker,
}

impl DetermineDriver<InspectDriverType> for Option<InspectDriverType> {
    fn determine_driver(&mut self) -> InspectDriverType {
        *self.get_or_insert(
            match (
                blue_build_utils::check_command_exists("skopeo"),
                blue_build_utils::check_command_exists("docker"),
                blue_build_utils::check_command_exists("podman"),
            ) {
                (Ok(_skopeo), _, _) => InspectDriverType::Skopeo,
                (_, Ok(_docker), _) => InspectDriverType::Docker,
                (_, _, Ok(_podman)) => InspectDriverType::Podman,
                _ => panic!(
                    "{}{}",
                    "Could not determine inspection strategy. ",
                    "You need either skopeo, docker, or podman",
                ),
            },
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BuildDriverType {
    Buildah,
    Podman,
    Docker,
}

impl DetermineDriver<BuildDriverType> for Option<BuildDriverType> {
    fn determine_driver(&mut self) -> BuildDriverType {
        *self.get_or_insert(
            match (
                blue_build_utils::check_command_exists("docker"),
                blue_build_utils::check_command_exists("podman"),
                blue_build_utils::check_command_exists("buildah"),
            ) {
                (Ok(_docker), _, _) if DockerDriver::is_supported_version() => {
                    BuildDriverType::Docker
                }
                (_, Ok(_podman), _) if PodmanDriver::is_supported_version() => {
                    BuildDriverType::Podman
                }
                (_, _, Ok(_buildah)) if BuildahDriver::is_supported_version() => {
                    BuildDriverType::Buildah
                }
                _ => panic!(
                    "{}{}{}{}",
                    "Could not determine strategy, ",
                    format_args!("need either docker version {}, ", DockerDriver::VERSION_REQ,),
                    format_args!("podman version {}, ", PodmanDriver::VERSION_REQ,),
                    format_args!(
                        "or buildah version {} to continue",
                        BuildahDriver::VERSION_REQ,
                    ),
                ),
            },
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SigningDriverType {
    Cosign,
    Podman,
    Docker,
}

impl DetermineDriver<SigningDriverType> for Option<SigningDriverType> {
    fn determine_driver(&mut self) -> SigningDriverType {
        trace!("SigningDriverType::determine_signing_driver()");

        *self.get_or_insert(
            match (
                blue_build_utils::check_command_exists("cosign"),
                blue_build_utils::check_command_exists("docker"),
                blue_build_utils::check_command_exists("podman"),
            ) {
                (Ok(_cosign), _, _) => SigningDriverType::Cosign,
                (_, Ok(_docker), _) => SigningDriverType::Docker,
                (_, _, Ok(_podman_driver)) => SigningDriverType::Podman,
                _ => panic!(concat!(
                    "Could not determine signing driver. ",
                    "Either cosign, docker, or podman ",
                    "is required to continue"
                )),
            },
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RunDriverType {
    Podman,
    Docker,
}

impl From<RunDriverType> for String {
    fn from(value: RunDriverType) -> Self {
        match value {
            RunDriverType::Podman => "podman".to_string(),
            RunDriverType::Docker => "docker".to_string(),
        }
    }
}

impl DetermineDriver<RunDriverType> for Option<RunDriverType> {
    fn determine_driver(&mut self) -> RunDriverType {
        trace!("RunDriver::determine_driver()");

        *self.get_or_insert(
            match (
                blue_build_utils::check_command_exists("docker"),
                blue_build_utils::check_command_exists("podman"),
            ) {
                (Ok(_docker), _) if DockerDriver::is_supported_version() => RunDriverType::Docker,
                (_, Ok(_podman)) if PodmanDriver::is_supported_version() => RunDriverType::Podman,
                _ => panic!(
                    "{}{}{}{}",
                    "Could not determine strategy, ",
                    format_args!("need either docker version {}, ", DockerDriver::VERSION_REQ),
                    format_args!("podman version {}, ", PodmanDriver::VERSION_REQ),
                    format_args!(
                        "or buildah version {} to continue",
                        BuildahDriver::VERSION_REQ
                    ),
                ),
            },
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CiDriverType {
    Local,
    Gitlab,
    Github,
}

impl DetermineDriver<CiDriverType> for Option<CiDriverType> {
    fn determine_driver(&mut self) -> CiDriverType {
        trace!("CiDriverType::determine_driver()");

        *self.get_or_insert(
            match (env::var(GITLAB_CI).ok(), env::var(GITHUB_ACTIONS).ok()) {
                (Some(_gitlab_ci), None) => CiDriverType::Gitlab,
                (None, Some(_github_actions)) => CiDriverType::Github,
                _ => CiDriverType::Local,
            },
        )
    }
}
