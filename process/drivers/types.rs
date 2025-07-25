use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use blue_build_utils::{
    constants::{GITHUB_ACTIONS, GITLAB_CI, IMAGE_VERSION_LABEL},
    get_env_var,
    semver::Version,
    string,
};
use clap::ValueEnum;
use log::{trace, warn};
use oci_distribution::Reference;
use serde::Deserialize;
use serde_json::Value;

use crate::drivers::{
    DriverVersion, buildah_driver::BuildahDriver, docker_driver::DockerDriver,
    podman_driver::PodmanDriver,
};

mod private {
    pub trait Private {}
}

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
    Sigstore,
}

impl DetermineDriver<SigningDriverType> for Option<SigningDriverType> {
    fn determine_driver(&mut self) -> SigningDriverType {
        trace!("SigningDriverType::determine_signing_driver()");

        *self.get_or_insert(
            blue_build_utils::check_command_exists("cosign")
                .map_or(SigningDriverType::Sigstore, |()| SigningDriverType::Cosign),
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
            match (
                get_env_var(GITLAB_CI).ok(),
                get_env_var(GITHUB_ACTIONS).ok(),
            ) {
                (Some(_gitlab_ci), None) => CiDriverType::Gitlab,
                (None, Some(_github_actions)) => CiDriverType::Github,
                _ => CiDriverType::Local,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Hash)]
pub enum Platform {
    #[value(name = "linux/amd64")]
    LinuxAmd64,

    #[value(name = "linux/amd64/v2")]
    LinuxAmd64V2,

    #[value(name = "linux/arm64")]
    LinuxArm64,

    #[value(name = "linux/arm")]
    LinuxArm,

    #[value(name = "linux/arm/v6")]
    LinuxArmV6,

    #[value(name = "linux/arm/v7")]
    LinuxArmV7,

    #[value(name = "linux/386")]
    Linux386,

    #[value(name = "linux/loong64")]
    LinuxLoong64,

    #[value(name = "linux/mips")]
    LinuxMips,

    #[value(name = "linux/mipsle")]
    LinuxMipsle,

    #[value(name = "linux/mips64")]
    LinuxMips64,

    #[value(name = "linux/mips64le")]
    LinuxMips64le,

    #[value(name = "linux/ppc64")]
    LinuxPpc64,

    #[value(name = "linux/ppc64le")]
    LinuxPpc64le,

    #[value(name = "linux/riscv64")]
    LinuxRiscv64,

    #[value(name = "linux/s390x")]
    LinuxS390x,
}

impl Platform {
    /// The architecture of the platform.
    #[must_use]
    pub const fn arch(&self) -> &str {
        match *self {
            Self::LinuxAmd64 | Self::LinuxAmd64V2 => "amd64",
            Self::LinuxArm64 => "arm64",
            Self::LinuxArm | Self::LinuxArmV6 | Self::LinuxArmV7 => "arm",
            Self::Linux386 => "386",
            Self::LinuxLoong64 => "loong64",
            Self::LinuxMips => "mips",
            Self::LinuxMipsle => "mipsle",
            Self::LinuxMips64 => "mips64",
            Self::LinuxMips64le => "mips64le",
            Self::LinuxPpc64 => "ppc64",
            Self::LinuxPpc64le => "ppc64le",
            Self::LinuxRiscv64 => "riscv64",
            Self::LinuxS390x => "s390x",
        }
    }

    /// The variant of the platform.
    #[must_use]
    pub const fn variant(&self) -> Option<&str> {
        match *self {
            Self::LinuxAmd64V2 => Some("v2"),
            Self::LinuxArmV6 => Some("v6"),
            Self::LinuxArmV7 => Some("v7"),
            _ => None,
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::LinuxAmd64 => "linux/amd64",
                Self::LinuxAmd64V2 => "linux/amd64/v2",
                Self::LinuxArm64 => "linux/arm64",
                Self::LinuxArm => "linux/arm",
                Self::LinuxArmV6 => "linux/arm/v6",
                Self::LinuxArmV7 => "linux/arm/v7",
                Self::Linux386 => "linux/386",
                Self::LinuxLoong64 => "linux/loong64",
                Self::LinuxMips => "linux/mips",
                Self::LinuxMipsle => "linux/mipsle",
                Self::LinuxMips64 => "linux/mips64",
                Self::LinuxMips64le => "linux/mips64le",
                Self::LinuxPpc64 => "linux/ppc64",
                Self::LinuxPpc64le => "linux/ppc64le",
                Self::LinuxRiscv64 => "linux/riscv64",
                Self::LinuxS390x => "linux/s390x",
            }
        )
    }
}

impl private::Private for Option<Platform> {}

pub trait PlatformInfo: private::Private {
    /// The string representation of the platform.
    ///
    /// If `None`, then the native architecture will be used.
    fn to_string(&self) -> String;

    /// The string representation of the architecture.
    ///
    /// If `None`, then the native architecture will be used.
    fn arch(&self) -> &str;
}

impl PlatformInfo for Option<Platform> {
    fn to_string(&self) -> String {
        self.map_or_else(
            || match std::env::consts::ARCH {
                "x86_64" => string!("linux/amd64"),
                "aarch64" => string!("linux/arm64"),
                arch => unimplemented!("Arch {arch} is unsupported"),
            },
            |platform| format!("{platform}"),
        )
    }

    fn arch(&self) -> &str {
        self.as_ref().map_or_else(
            || match std::env::consts::ARCH {
                "x86_64" => "amd64",
                "aarch64" => "arm64",
                arch => unimplemented!("Arch {arch} is unsupported"),
            },
            Platform::arch,
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
                .and_then(|v| {
                    serde_json::from_value::<Version>(v)
                        .inspect_err(|e| warn!("Failed to parse version:\n{e}"))
                        .ok()
                })?
                .major,
        )
    }
}

#[derive(Debug, Clone)]
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

pub struct MountId(pub(super) String);

impl std::fmt::Display for MountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for MountId {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

impl<'a> From<&'a MountId> for std::borrow::Cow<'a, str> {
    fn from(value: &'a MountId) -> Self {
        Self::Borrowed(&value.0)
    }
}

#[derive(Debug, Clone)]
pub struct OciDir(String);

impl std::fmt::Display for OciDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for OciDir {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

impl TryFrom<std::path::PathBuf> for OciDir {
    type Error = miette::Report;

    fn try_from(value: std::path::PathBuf) -> Result<Self, Self::Error> {
        if !value.is_dir() {
            miette::bail!("OCI directory doesn't exist at {}", value.display());
        }

        Ok(Self(format!("oci:{}", value.display())))
    }
}

/// An image ref that could reference
/// a remote registry or a local tarball.
#[derive(Debug, Clone)]
pub enum ImageRef<'scope> {
    Remote(Cow<'scope, Reference>),
    LocalTar(Cow<'scope, Path>),
}

impl ImageRef<'_> {
    #[must_use]
    pub fn remote_ref(&self) -> Option<&Reference> {
        match self {
            Self::Remote(remote) => Some(remote.as_ref()),
            Self::LocalTar(_) => None,
        }
    }
}

impl<'scope> From<&'scope Self> for ImageRef<'scope> {
    fn from(value: &'scope ImageRef) -> Self {
        match value {
            Self::Remote(remote) => Self::Remote(Cow::Borrowed(remote.as_ref())),
            Self::LocalTar(path) => Self::LocalTar(Cow::Borrowed(path.as_ref())),
        }
    }
}

impl<'scope> From<&'scope Reference> for ImageRef<'scope> {
    fn from(value: &'scope Reference) -> Self {
        Self::Remote(Cow::Borrowed(value))
    }
}

impl From<Reference> for ImageRef<'_> {
    fn from(value: Reference) -> Self {
        Self::Remote(Cow::Owned(value))
    }
}

impl<'scope> From<&'scope Path> for ImageRef<'scope> {
    fn from(value: &'scope Path) -> Self {
        Self::LocalTar(Cow::Borrowed(value))
    }
}

impl<'scope> From<&'scope PathBuf> for ImageRef<'scope> {
    fn from(value: &'scope PathBuf) -> Self {
        Self::from(value.as_path())
    }
}

impl From<PathBuf> for ImageRef<'_> {
    fn from(value: PathBuf) -> Self {
        Self::LocalTar(Cow::Owned(value))
    }
}

impl From<ImageRef<'_>> for String {
    fn from(value: ImageRef<'_>) -> Self {
        Self::from(&value)
    }
}

impl From<&ImageRef<'_>> for String {
    fn from(value: &ImageRef<'_>) -> Self {
        format!("{value}")
    }
}

impl std::fmt::Display for ImageRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Remote(remote) => remote.whole(),
                Self::LocalTar(path) => format!("oci-archive:{}", path.display()),
            }
        )
    }
}
