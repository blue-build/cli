use bon::Builder;
use oci_distribution::Reference;

use crate::drivers::types::Platform;

#[derive(Debug, Clone, Builder, Hash)]
#[builder(derive(Clone))]
pub struct GetMetadataOpts<'scope> {
    #[builder(into)]
    pub image: &'scope Reference,

    pub platform: Option<Platform>,
}
