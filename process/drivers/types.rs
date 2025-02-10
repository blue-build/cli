use std::{collections::HashMap, env};

use blue_build_utils::{
    constants::{GITHUB_ACTIONS, GITLAB_CI, IMAGE_VERSION_LABEL},
    semver::Version,
};
use clap::ValueEnum;
use log::trace;
use serde::Deserialize;
use serde_json::Value;

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
                (Ok(_docker), _, _)
                    if DockerDriver::is_supported_version() && DockerDriver::has_buildx() =>
                {
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
                    format_args!(
                        "need either docker version {} with buildx, ",
                        DockerDriver::VERSION_REQ,
                    ),
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
    #[cfg(feature = "sigstore")]
    Sigstore,
}

impl DetermineDriver<SigningDriverType> for Option<SigningDriverType> {
    fn determine_driver(&mut self) -> SigningDriverType {
        trace!("SigningDriverType::determine_signing_driver()");

        #[cfg(feature = "sigstore")]
        {
            *self.get_or_insert(
                blue_build_utils::check_command_exists("cosign")
                    .map_or(SigningDriverType::Sigstore, |()| SigningDriverType::Cosign),
            )
        }

        #[cfg(not(feature = "sigstore"))]
        {
            SigningDriverType::Cosign
        }
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

#[derive(Debug, Default, Clone, Copy, ValueEnum, PartialEq, Eq, Hash)]
pub enum Platform {
    #[default]
    #[value(name = "native")]
    Native,
    #[value(name = "linux/amd64")]
    LinuxAmd64,

    #[value(name = "linux/arm64")]
    LinuxArm64,
}

impl Platform {
    /// The architecture of the platform.
    #[must_use]
    pub fn arch(&self) -> &str {
        match *self {
            Self::Native => match std::env::consts::ARCH {
                "x86_64" => "amd64",
                "aarch64" => "arm64",
                arch => unimplemented!("Arch {arch} is unsupported"),
            },
            Self::LinuxAmd64 => "amd64",
            Self::LinuxArm64 => "arm64",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Native => match std::env::consts::ARCH {
                    "x86_64" => "linux/amd64",
                    "aarch64" => "linux/arm64",
                    arch => unimplemented!("Arch {arch} is unsupported"),
                },
                Self::LinuxAmd64 => "linux/amd64",
                Self::LinuxArm64 => "linux/arm64",
            }
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ImageMetadata {
    pub labels: HashMap<String, Value>,
    pub digest: String,
}

impl ImageMetadata {
    #[must_use]
    pub fn get_version(&self) -> Option<u64> {
        Some(
            self.labels
                .get(IMAGE_VERSION_LABEL)
                .map(ToOwned::to_owned)
                .and_then(|v| serde_json::from_value::<Version>(v).ok())?
                .major,
        )
    }
}

pub struct ContainerId(pub(super) String);

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for ContainerId {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

#[cfg(feature = "rechunk")]
pub struct MountId(pub(super) String);

#[cfg(feature = "rechunk")]
impl std::fmt::Display for MountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[cfg(feature = "rechunk")]
impl AsRef<std::ffi::OsStr> for MountId {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

#[cfg(feature = "rechunk")]
impl<'a> From<&'a MountId> for std::borrow::Cow<'a, str> {
    fn from(value: &'a MountId) -> Self {
        Self::Borrowed(&value.0)
    }
}

#[cfg(feature = "rechunk")]
pub struct OciDir(String);

#[cfg(feature = "rechunk")]
impl std::fmt::Display for OciDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[cfg(feature = "rechunk")]
impl AsRef<std::ffi::OsStr> for OciDir {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

#[cfg(feature = "rechunk")]
impl TryFrom<std::path::PathBuf> for OciDir {
    type Error = miette::Report;

    fn try_from(value: std::path::PathBuf) -> Result<Self, Self::Error> {
        if !value.is_dir() {
            miette::bail!("OCI directory doesn't exist at {}", value.display());
        }

        Ok(Self(format!("oci:{}", value.display())))
    }
}
