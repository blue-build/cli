use blue_build_utils::semver::Version;
use serde::{Deserialize, Serialize, de::Error};

#[derive(Default, Clone, Debug)]
pub enum MaybeVersion {
    #[default]
    None,
    Version(Version),
}

impl std::fmt::Display for MaybeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none".to_string(),
                Self::Version(version) => version.to_string(),
            }
        )
    }
}

impl<'de> Deserialize<'de> for MaybeVersion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = String::deserialize(deserializer)?;

        Ok(match val {
            none if none.to_lowercase() == "none" => Self::None,
            version => Self::Version(
                version
                    .parse()
                    .map_err(|e: miette::Error| D::Error::custom(e.to_string()))?,
            ),
        })
    }
}

impl Serialize for MaybeVersion {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
