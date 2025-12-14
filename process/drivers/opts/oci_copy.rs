use blue_build_utils::container::OciSource;
use bon::Builder;
use oci_client::Reference;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct CopyOciSourceOpts<'scope> {
    pub oci_source: &'scope OciSource,
    pub registry: &'scope Reference,

    #[builder(default)]
    pub privileged: bool,
}
