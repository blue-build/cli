use clap::ValueEnum;

pub use build::*;
pub use inspect::*;

mod build;
mod inspect;

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
