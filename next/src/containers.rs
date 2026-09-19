use std::{convert::Infallible, ffi::OsStr, str::FromStr};

use blue_build_utils::{cmd_out, platform::Platform};
use miette::Result;
use serde::Deserialize;

use crate::layer::{FromLayerReference, LayerId};

#[derive(Debug, PartialEq, Eq, Hash, Deserialize)]
pub struct Container(String);

#[bon::bon]
impl Container {
    #[builder(finish_fn = "build")]
    pub fn from_image(image: &FromLayerReference, platform: Option<Platform>) -> Result<Self> {
        cmd_out!(
            parse = Self;
            err_msg = format!(
                "Failed to create FROM {image}"
            );
            "buildah",
            "from",
            if let Some(platform) = platform => format!("--platform={platform}"),
            format!("docker://{image}"),
        )
    }

    pub fn from_layer(id: &LayerId) -> Result<Self> {
        cmd_out!(
            parse = Self;
            err_msg = format!("Unable to create a new container from {id}");
            "buildah",
            "from",
            id,
        )
    }

    pub fn remove(self) -> Result<()> {
        cmd_out!(
            err_msg = format!("Failed to remove container {self}");
            "buildah",
            "rm",
            &self
        )
    }
}

impl AsRef<OsStr> for Container {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl std::fmt::Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Container {
    type Err = Infallible;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(Self(s.trim().to_string()))
    }
}
