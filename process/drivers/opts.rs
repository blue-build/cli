use clap::ValueEnum;

pub use build::*;
pub use ci::*;
pub use inspect::*;
#[cfg(feature = "rechunk")]
pub use rechunk::*;
pub use run::*;
pub use signing::*;

mod build;
mod ci;
mod inspect;
#[cfg(feature = "rechunk")]
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
