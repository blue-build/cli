use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::Platform;

#[derive(Debug, Clone, Copy, Builder, Hash)]
#[builder(derive(Clone))]
pub struct GetMetadataOpts<'scope> {
    #[builder(into)]
    pub image: &'scope Reference,

    #[builder(default)]
    pub platform: Platform,
}
