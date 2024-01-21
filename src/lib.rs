//! The root library for blue-build.

#![doc(
    html_logo_url = "https://gitlab.com/wunker-bunker/blue-build/-/raw/main/logos/BlueBuild-logo.png"
)]
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(unused_imports)]

#[cfg(feature = "init")]
pub mod init;

pub mod build;
pub mod local;
mod ops;
pub mod template;
