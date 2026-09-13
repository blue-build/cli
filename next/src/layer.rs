mod build_step;
mod copy;
mod from;
mod mount;
mod run;

use std::{ffi::OsStr, hash::Hash, str::FromStr, sync::Arc};

use blue_build_utils::cmd_out;
use lazy_regex::regex;
use miette::{Diagnostic, Result};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    containers::Container,
    layer::build_step::{BuildLayer, Buildable},
};

pub use build_step::FinalizeLayer;
pub use copy::*;
pub use from::*;
pub use mount::*;
pub use run::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Layer {
    From(Arc<FromLayer>),
    Run(Arc<RunLayer>),
    Copy(Arc<CopyLayer>),
}

impl BuildLayer for Layer {
    fn build(&self) -> Result<Box<dyn Buildable>> {
        match self {
            Self::From(layer) => layer.build(),
            Self::Run(layer) => layer.build(),
            Self::Copy(layer) => layer.build(),
        }
    }

    fn run_step(&self, container: &Container) -> Result<()> {
        match self {
            Self::From(layer) => layer.run_step(container),
            Self::Run(layer) => layer.run_step(container),
            Self::Copy(layer) => layer.run_step(container),
        }
    }
}

impl FinalizeLayer for Layer {}

macro_rules! layers {
    ($($typ:ty => $var:ident),* $(,)?) => {
        $(
        impl From<Arc<$typ>> for Layer {
            fn from(value: Arc<$typ>) -> Self {
                Self::$var(value)
            }
        }

        impl From<$typ> for Layer {
            fn from(value: $typ) -> Self {
                Self::$var(Arc::new(value))
            }
        }
        )*
    };
}

layers!(
    FromLayer => From,
    RunLayer => Run,
    CopyLayer => Copy,
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(String);

#[bon::bon]
impl LayerId {
    #[builder]
    pub fn new(container: Container) -> Result<Arc<Self>> {
        let layer = cmd_out!(
            parse = Self;
            err_msg = format!("Failed to commit layer for {container}");
            "buildah",
            "commit",
            &container,
        )
        .map(Arc::new)?;
        container.remove()?;
        Ok(layer)
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("Invalid layer ID {}", .0)]
pub struct LayerIdParseError(String);

impl FromStr for LayerId {
    type Err = LayerIdParseError;

    fn from_str(value: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let value = value.trim();

        if regex!("^[a-f0-9]{64}$").is_match(value) {
            Ok(Self(value.trim().to_string()))
        } else {
            Err(LayerIdParseError(value.to_string()))
        }
    }
}

impl std::fmt::Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for LayerId {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .trim()
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl AsRef<OsStr> for LayerId {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}
