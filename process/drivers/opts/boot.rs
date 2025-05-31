use bon::Builder;
use oci_distribution::Reference;

#[derive(Debug, Clone, Copy, Builder)]
pub struct SwitchOpts<'scope> {
    pub image: &'scope Reference,

    #[builder(default)]
    pub reboot: bool,
}
