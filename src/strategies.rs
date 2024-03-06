use anyhow::Result;
use typed_builder::TypedBuilder;

pub mod buildah_strategy;
pub mod docker_strategy;
#[cfg(feature = "builtin-podman")]
pub mod podman_api_strategy;
pub mod podman_strategy;

#[derive(Debug, Default, Clone, TypedBuilder)]
pub struct Credentials {
    pub registry: String,
    pub username: String,
    pub password: String,
}

pub trait BuildStrategy: Sync + Send {
    fn build(&self, image: &str) -> Result<()>;

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()>;

    fn push(&self, image: &str) -> Result<()>;

    fn login(&self) -> Result<()>;

    fn inspect(&self, image_name: &str, tag: &str) -> Result<Vec<u8>>;
}
