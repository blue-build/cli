use std::str::FromStr;

use miette::bail;
use semver::Prerelease;
use serde::{Deserialize, Serialize, de::Error};

#[derive(Debug, Clone, Serialize)]
pub struct Version(semver::Version);

impl std::ops::Deref for Version {
    type Target = semver::Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ver = String::deserialize(deserializer)?;
        ver.parse()
            .map_err(|e: miette::Error| D::Error::custom(e.to_string()))
    }
}

impl FromStr for Version {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Ok(mut parsed_ver) = lenient_semver::parse(s) else {
            bail!("Failed to deserialize version {s}");
        };
        // delete pre-release field or we can never match pre-release versions of tools
        parsed_ver.pre = Prerelease::EMPTY;
        Ok(Self(parsed_ver))
    }
}
