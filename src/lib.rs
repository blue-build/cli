//! The root library for blue-build.

#[cfg(feature = "init")]
pub mod init;

#[cfg(feature = "build")]
pub mod build;

pub mod module_recipe;
mod ops;
pub mod template;
