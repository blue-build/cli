//! The root library for blue-build.

#[cfg(feature = "init")]
pub mod init;

#[cfg(feature = "build")]
pub mod build;

mod ops;
pub mod template;
