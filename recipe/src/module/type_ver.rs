use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone)]
pub struct ModuleTypeVersion<'scope> {
    typ: Cow<'scope, str>,
    version: Option<Cow<'scope, str>>,
}

impl ModuleTypeVersion<'_> {
    #[must_use]
    pub fn typ(&self) -> &str {
        &self.typ
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

impl std::fmt::Display for ModuleTypeVersion<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.version.as_deref() {
            Some(version) => {
                write!(f, "{}@{version}", &self.typ)
            }
            None => {
                write!(f, "{}", &self.typ)
            }
        }
    }
}

impl<'scope> From<&'scope str> for ModuleTypeVersion<'scope> {
    fn from(s: &'scope str) -> Self {
        if let Some((typ, version)) = s.split_once('@') {
            Self {
                typ: Cow::Borrowed(typ),
                version: Some(Cow::Borrowed(version)),
            }
        } else {
            Self {
                typ: Cow::Borrowed(s),
                version: None,
            }
        }
    }
}

impl From<String> for ModuleTypeVersion<'_> {
    fn from(s: String) -> Self {
        if let Some((typ, version)) = s.split_once('@') {
            Self {
                typ: Cow::Owned(typ.to_owned()),
                version: Some(Cow::Owned(version.to_owned())),
            }
        } else {
            Self {
                typ: Cow::Owned(s),
                version: None,
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
