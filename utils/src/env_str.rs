use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq)]
pub struct EnvString {
    unexpanded: String,
    expanded: String,
}

impl<'de> Deserialize<'de> for EnvString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let unexpanded = String::deserialize(deserializer)?;
        let expanded = shellexpand::env(&unexpanded)
            .map_err(serde::de::Error::custom)?
            .into();

        Ok(Self {
            unexpanded,
            expanded,
        })
    }
}

impl Serialize for EnvString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.unexpanded)
    }
}

impl std::fmt::Display for EnvString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expanded)
    }
}

impl std::ops::Deref for EnvString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.expanded.as_str()
    }
}

impl From<String> for EnvString {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&String> for EnvString {
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

/// Context function for env var expansion.
/// If the variable doesn't exist, return None to prevent expansion.
fn context(var: &str) -> Option<String> {
    crate::get_env_var(var).ok()
}

impl From<&str> for EnvString {
    fn from(value: &str) -> Self {
        Self {
            unexpanded: value.to_string(),
            expanded: shellexpand::env_with_context_no_errors(&value, context).to_string(),
        }
    }
}

impl From<EnvString> for String {
    fn from(value: EnvString) -> Self {
        value.expanded
    }
}

impl PartialEq for EnvString {
    fn eq(&self, other: &Self) -> bool {
        self.expanded.eq(&other.expanded)
    }
}

impl Ord for EnvString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.expanded.cmp(&other.expanded)
    }
}

impl PartialOrd for EnvString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
