use std::path::Path;

use bon::Builder;
use oci_client::Reference;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct InspectImageOpts<'scope> {
    /// Image in local storage to be inspected
    pub image: &'scope str,

    /// Path to write output, or `None` if output should be returned to the caller.
    pub output_path: Option<&'scope Path>,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct RemoveImageOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}
