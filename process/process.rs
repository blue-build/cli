//! This module is responsible for managing processes spawned
//! by this tool. It contains drivers for running, building, inspecting, and signing
//! images that interface with tools like docker or podman.

#[cfg(feature = "sigstore")]
use once_cell::sync::Lazy;
#[cfg(feature = "sigstore")]
use tokio::runtime::Runtime;

pub mod drivers;
pub mod logging;
pub mod signal_handler;

#[cfg(feature = "sigstore")]
pub(crate) static RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

#[cfg(test)]
pub(crate) mod test {
    use std::sync::LazyLock;

    pub const TEST_TAG_1: &str = "test-tag-1";
    pub const TEST_TAG_2: &str = "test-tag-2";

    pub static TIMESTAMP: LazyLock<String> = LazyLock::new(blue_build_utils::get_tag_timestamp);
}
