//! The root library for blue-build.
#![doc = include_str!("../README.md")]
#![allow(clippy::needless_raw_string_hashes)]

pub(crate) mod info {
    #![allow(clippy::too_long_first_doc_paragraph)]
    shadow_rs::shadow!(shadow);
}

pub mod commands;
pub mod rpm_ostree_status;
