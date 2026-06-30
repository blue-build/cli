use clap::ValueEnum;

pub use boot::*;
pub use build::*;
pub use build_chunked_oci::*;
pub use ci::*;
pub use image_storage::*;
pub use inspect::*;
pub use oci_copy::*;
pub use post_build::*;
pub use rechunk::*;
pub use run::*;
pub use signing::*;

mod boot;
mod build;
mod build_chunked_oci;
mod ci;
mod image_storage;
mod inspect;
mod oci_copy;
mod post_build;
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
