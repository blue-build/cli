use std::num::NonZero;

use bon::Builder;

use super::BuildTagPushOpts;

#[derive(Debug, Clone, Builder)]
#[builder(derive(Debug, Clone))]
#[non_exhaustive]
pub struct BuildRechunkTagPushOpts<'scope> {
    pub build_tag_push_opts: BuildTagPushOpts<'scope>,
    pub rechunk_opts: BuildChunkedOciOpts,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
#[non_exhaustive]
pub struct BuildChunkedOciOpts {
    /// Format version for `build-chunked-oci`. Currently must be either `1` or `2`.
    #[builder(default = 2)]
    pub format_version: u32,

    /// Maximum number of layers to use. Currently defaults to 64 if not specified.
    pub max_layers: Option<NonZero<u32>>,
}
