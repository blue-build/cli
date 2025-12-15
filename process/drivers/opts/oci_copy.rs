use blue_build_utils::container::OciRef;
use bon::Builder;

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct CopyOciOpts<'scope> {
    pub src_ref: &'scope OciRef,
    pub dest_ref: &'scope OciRef,

    #[builder(default)]
    pub privileged: bool,

    #[builder(default)]
    pub retry_count: u8,
}
