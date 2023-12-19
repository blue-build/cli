//! The root library for ublue-rs.
//!
//! This module consists of the args for the cli as well as the
//! initial entrypoint for setting up tera to properly template
//! the Containerfile. There is support for legacy starting point
//! recipes using the feature flag 'legacy' and support for the newest
//! starting point setup using the 'modules' feature flag. You will not want
//! to use both features at the same time. For now the 'legacy' feature
//! is the default feature until modules works 1-1 with ublue starting point.

#[cfg(feature = "init")]
pub mod init;

#[cfg(feature = "build")]
pub mod build;

pub mod module_recipe;
mod ops;
pub mod template;
