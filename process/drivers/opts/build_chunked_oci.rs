use std::num::NonZeroU32;

use blue_build_utils::{constants::DEFAULT_MAX_LAYERS, platform::Platform};
use bon::Builder;

use super::BuildTagPushOpts;

#[derive(Debug, Clone, Builder)]
#[builder(derive(Debug, Clone))]
pub struct BuildRechunkTagPushOpts<'scope> {
    pub build_tag_push_opts: BuildTagPushOpts<'scope>,
    pub rechunk_opts: BuildChunkedOciOpts,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct BuildChunkedOciOpts {
    /// Format version for `build-chunked-oci`.
    #[builder(default = BuildChunkedOciFormatVersion::V2)]
    pub format_version: BuildChunkedOciFormatVersion,

    /// Maximum number of layers to use. Currently defaults to 64 if not specified.
    #[builder(default = DEFAULT_MAX_LAYERS)]
    pub max_layers: NonZeroU32,

    /// Build layer plan from scratch instead of using the previous build as a baseline.
    #[builder(default)]
    pub clear_plan: bool,

    /// Platform to use for baseline previous build.
    pub platform: Option<Platform>,
}

impl BuildChunkedOciOpts {
    #[must_use]
    pub const fn with_platform(self, platform: Platform) -> Self {
        Self {
            platform: Some(platform),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BuildChunkedOciFormatVersion {
    V1,
    V2,
}

impl std::fmt::Display for BuildChunkedOciFormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::V1 => "1",
                Self::V2 => "2",
            }
        )
    }
}
