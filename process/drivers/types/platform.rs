use clap::ValueEnum;

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
