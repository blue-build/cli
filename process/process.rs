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
    use std::sync::Mutex;

    use blue_build_recipe::{Module, ModuleExt, Recipe};
    use indexmap::IndexMap;
    use once_cell::sync::Lazy;

    pub const BB_UNIT_TEST_MOCK_GET_OS_VERSION: &str = "BB_UNIT_TEST_MOCK_GET_OS_VERSION";

    /// This mutex is used for tests that require the reading of
    /// environment variables. Env vars are an inheritly unsafe
    /// as they can be changed and affect other threads functionality.
    ///
    /// For tests that require setting env vars, they need to lock this
    /// mutex before making changes to the env. Any changes made to the env
    /// MUST be undone in the same test before dropping the lock. Failure to
    /// do so will cause unpredictable behavior with other tests.
    pub static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    pub fn create_test_recipe() -> Recipe<'static> {
        Recipe::builder()
            .name("test")
            .description("This is a test")
            .base_image("base-image")
            .image_version("40")
            .modules_ext(
                ModuleExt::builder()
                    .modules(vec![Module::builder().build()])
                    .build(),
            )
            .stages_ext(None)
            .extra(IndexMap::new())
            .build()
    }
}
