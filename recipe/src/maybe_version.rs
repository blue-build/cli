use blue_build_utils::container::Tag;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug)]
pub enum MaybeVersion {
    #[default]
    None,
    VersionOrBranch(Tag),
}

impl std::fmt::Display for MaybeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::VersionOrBranch(version) => version.as_str(),
            }
        )
    }
}

impl<'de> Deserialize<'de> for MaybeVersion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = Tag::deserialize(deserializer)?;

        Ok(match val {
            none if none.as_str().to_lowercase() == "none" => Self::None,
            version => Self::VersionOrBranch(version),
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
