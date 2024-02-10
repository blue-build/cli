//! The root library for blue-build.
#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::perf,
    clippy::style,
    clippy::nursery
)]
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

shadow_rs::shadow!(shadow);

pub mod commands;
pub mod constants;
pub mod module_recipe;
mod ops;
