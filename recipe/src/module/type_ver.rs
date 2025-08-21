use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone)]
pub struct ModuleTypeVersion {
    typ: String,
    version: Option<String>,
}

impl ModuleTypeVersion {
    #[must_use]
    pub fn typ(&self) -> &str {
        self.typ.as_ref()
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

impl std::fmt::Display for ModuleTypeVersion {
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

impl From<&str> for ModuleTypeVersion {
    fn from(s: &str) -> Self {
        if let Some((typ, version)) = s.split_once('@') {
            Self {
                typ: typ.into(),
                version: Some(version.into()),
            }
        } else {
            Self {
                typ: s.into(),
                version: None,
            }
        }
    }
}

impl From<String> for ModuleTypeVersion {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl Serialize for ModuleTypeVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ModuleTypeVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: String = Deserialize::deserialize(deserializer)?;
        Ok(value.into())
    }
}
