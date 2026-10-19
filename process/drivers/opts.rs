use bon::Builder;
use clap::ValueEnum;

pub use boot::*;
#[cfg(not(feature = "next-builder"))]
pub use build::*;
#[cfg(not(feature = "next-builder"))]
pub use build_chunked_oci::*;
pub use ci::*;
pub use image_storage::*;
pub use inspect::*;
#[cfg(not(feature = "next-builder"))]
pub use oci_copy::*;
#[cfg(not(feature = "next-builder"))]
pub use post_build::*;
#[cfg(not(feature = "next-builder"))]
pub use rechunk::*;
pub use run::*;
pub use signing::*;

mod boot;
#[cfg(not(feature = "next-builder"))]
mod build;
#[cfg(not(feature = "next-builder"))]
mod build_chunked_oci;
mod ci;
mod image_storage;
mod inspect;
#[cfg(not(feature = "next-builder"))]
mod oci_copy;
#[cfg(not(feature = "next-builder"))]
mod post_build;
#[cfg(not(feature = "next-builder"))]
mod rechunk;
mod run;
mod signing;

#[derive(Debug, Copy, Clone, Default, ValueEnum)]
pub enum CompressionType {
    #[default]
    Gzip,
    Zstd,
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Zstd => "zstd",
            Self::Gzip => "gzip",
        })
    }
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct PruneOpts {
    #[builder(default)]
    pub all: bool,

    #[builder(default)]
    pub volumes: bool,
}
