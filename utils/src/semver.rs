use std::{ops::Not, str::FromStr};

use lazy_regex::regex_captures;
use miette::bail;
use semver::{BuildMetadata, Prerelease};
use serde::{Deserialize, Serialize, de::Error};

#[derive(Debug, Clone, Serialize)]
pub struct Version {
    prefix: Option<String>,
    version: semver::Version,
}

impl std::ops::Deref for Version {
    type Target = semver::Version;

    fn deref(&self) -> &Self::Target {
        &self.version
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.version.fmt(f)
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
        let Some((_whole, prefix, major, minor, patch)) = regex_captures!(
            r"(?:(?<prefix>[a-zA-Z0-9_-]+)-)?v?(?<major>\d+)(?:\.(?<minor>\d+))?(?:\.(?<patch>\d+))?(?:[-_].*)?",
            s.trim()
        ) else {
            bail!("Failed to parse version {s} with regex");
        };

        let Ok(mut version) = semver::Version::parse(&format!(
            "{major}.{minor}.{patch}",
            minor = if minor.is_empty() { "0" } else { minor },
            patch = if patch.is_empty() { "0" } else { patch }
        )) else {
            bail!("Failed to parse version {s}");
        };

        // delete pre-release field or we can never match pre-release versions of tools
        version.pre = Prerelease::EMPTY;
        version.build = BuildMetadata::EMPTY;

        let prefix = prefix.is_empty().not().then(|| prefix.into());

        Ok(Self { prefix, version })
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::semver::Version;

    #[rstest]
    #[case("42", None, 42, 0, 0)]
    #[case("42.20251020.1", None, 42, 20_251_020, 1)]
    #[case("42.20251020", None, 42, 20_251_020, 0)]
    #[case("latest-42.20251020.1", Some("latest"), 42, 20_251_020, 1)]
    #[case("42-beta.0", None, 42, 0, 0)]
    #[case("stable-42-beta.0", Some("stable"), 42, 0, 0)]
    #[case("v42", None, 42, 0, 0)]
    #[case("v42.20251020.1", None, 42, 20_251_020, 1)]
    #[case("v42.20251020", None, 42, 20_251_020, 0)]
    #[case("latest-v42.20251020.1", Some("latest"), 42, 20_251_020, 1)]
    #[case("v42-beta.0", None, 42, 0, 0)]
    #[case("stable-v42-beta.0", Some("stable"), 42, 0, 0)]
    fn parse_version(
        #[case] version: &str,
        #[case] prefix: Option<&str>,
        #[case] major: u64,
        #[case] minor: u64,
        #[case] patch: u64,
    ) {
        let version = version.parse::<Version>().unwrap();
        assert_eq!(version.prefix.as_deref(), prefix);
        assert_eq!(version.version.major, major);
        assert_eq!(version.version.minor, minor);
        assert_eq!(version.version.patch, patch);
    }
}
