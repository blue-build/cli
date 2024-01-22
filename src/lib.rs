//! The root library for blue-build.
#![warn(clippy::correctness, clippy::suspicious, clippy::perf, clippy::style)]
#![doc(
    html_logo_url = "https://gitlab.com/wunker-bunker/blue-build/-/raw/main/logos/BlueBuild-logo.png"
)]
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(unused_imports)]
#![allow(clippy::module_name_repetitions)]

pub mod commands;
mod ops;
