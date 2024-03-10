//! The root library for blue-build.
#![doc = include_str!("../README.md")]

shadow_rs::shadow!(shadow);

pub mod commands;
pub mod credentials;
pub mod image_inspection;
pub mod strategies;
