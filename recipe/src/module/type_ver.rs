use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone)]
pub struct ModuleTypeVersion<'scope> {
    typ: Cow<'scope, str>,
    version: Cow<'scope, str>,
}

impl<'scope> ModuleTypeVersion<'scope> {
    #[must_use]
    pub fn typ(&self) -> &str {
        &self.typ
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl std::fmt::Display for ModuleTypeVersion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", &self.typ, &self.version)
    }
}

impl<'scope> From<&'scope str> for ModuleTypeVersion<'scope> {
    fn from(s: &'scope str) -> Self {
        if let Some((typ, version)) = s.split_once('@') {
            Self {
                typ: Cow::Borrowed(typ),
                version: Cow::Borrowed(version),
            }
        } else {
            Self {
                typ: Cow::Borrowed(s),
                version: Cow::Owned("latest".into()),
            }
        }
    }
}

impl From<String> for ModuleTypeVersion<'_> {
    fn from(s: String) -> Self {
        if let Some((typ, version)) = s.split_once('@') {
            Self {
                typ: Cow::Owned(typ.to_owned()),
                version: Cow::Owned(version.to_owned()),
            }
        } else {
            Self {
                typ: Cow::Owned(s),
                version: Cow::Owned("latest".into()),
            }
        }
    }
}

impl Serialize for ModuleTypeVersion<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ModuleTypeVersion<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: String = Deserialize::deserialize(deserializer)?;
        Ok(value.into())
    }
}
