use semver::Prerelease;
use serde::{de::Error, Deserialize};

#[derive(Debug)]
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
        let Ok(mut parsed_ver) = lenient_semver::parse(&ver) else {
            return Err(D::Error::custom(format!(
                "Failed to deserialize version {ver}"
            )));
        };
        // delete pre-release field or we can never match pre-release versions of tools
        parsed_ver.pre = Prerelease::EMPTY;
        Some(Self(parsed_ver))
            .ok_or_else(|| D::Error::custom(format!("Failed to deserialize version {ver}")))
    }
}
