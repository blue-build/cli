//! The root library for blue-build.
#![doc = include_str!("../README.md")]
#![allow(clippy::needless_raw_string_hashes)]

shadow_rs::shadow!(shadow);

pub mod commands;
pub mod image_inspection;
pub mod strategies;
