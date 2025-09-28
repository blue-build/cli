use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use clap::ValueEnum;
use miette::bail;
use oci_distribution::Reference;
use serde::{Deserialize, Serialize};

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

    /// Get a tag friendly version of the platform.
    #[must_use]
    pub fn tagged_image(&self, image: &Reference) -> Reference {
        Reference::with_tag(
            image.registry().to_string(),
            image.repository().to_string(),
            format!("{}_{self}", image.tag().unwrap_or("latest")).replace('/', "_"),
        )
    }

    /// Get a tagged path.
    #[must_use]
    pub fn tagged_path(&self, path: &Path) -> Option<PathBuf> {
        if let Some(file_stem) = path.file_stem()
            && let Some(extension) = path.extension()
        {
            Some(
                path.with_file_name(format!(
                    "{}_{}",
                    file_stem.display(),
                    self.to_string().replace('/', "_")
                ))
                .with_extension(extension),
            )
        } else {
            None
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        match std::env::consts::ARCH {
            "x86_64" => Self::LinuxAmd64,
            "aarch64" => Self::LinuxArm64,
            "x86" => Self::Linux386,
            "arm" => Self::LinuxArm,
            "mips" => Self::LinuxMips,
            "mips32r6" => Self::LinuxMipsle,
            "mips64" => Self::LinuxMips64,
            "mips64r6" => Self::LinuxMips64le,
            "powerpc64" => Self::LinuxPpc64,
            "riscv64" => Self::LinuxRiscv64,
            "s390x" => Self::LinuxS390x,
            "loongarch64" => Self::LinuxLoong64,
            arch => unimplemented!("Arch {arch} is unsupported"),
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

impl FromStr for Platform {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "linux/amd64" => Self::LinuxAmd64,
            "linux/amd64/v2" => Self::LinuxAmd64V2,
            "linux/arm64" => Self::LinuxArm64,
            "linux/arm" => Self::LinuxArm,
            "linux/arm/v6" => Self::LinuxArmV6,
            "linux/arm/v7" => Self::LinuxArmV7,
            "linux/386" => Self::Linux386,
            "linux/loong64" => Self::LinuxLoong64,
            "linux/mips" => Self::LinuxMips,
            "linux/mipsle" => Self::LinuxMipsle,
            "linux/mips64" => Self::LinuxMips64,
            "linux/mips64le" => Self::LinuxMips64le,
            "linux/ppc64" => Self::LinuxPpc64,
            "linux/ppc64le" => Self::LinuxPpc64le,
            "linux/riscv64" => Self::LinuxRiscv64,
            "linux/s390x" => Self::LinuxS390x,
            platform => bail!("Platform {platform} unsupported"),
        })
    }
}

impl<'de> Deserialize<'de> for Platform {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
