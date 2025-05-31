use std::{borrow::Cow, path::PathBuf};

use blue_build_utils::constants::OCI_ARCHIVE;
use log::warn;
use oci_distribution::Reference;
use serde::Deserialize;

use crate::drivers::{BootStatus, types::ImageRef};

#[derive(Deserialize, Debug, Clone)]
pub struct BootcStatus {
    status: BootcStatusExt,
}

#[derive(Deserialize, Debug, Clone)]
struct BootcStatusExt {
    staged: Option<BootcStatusImage>,
    booted: BootcStatusImage,
}

#[derive(Deserialize, Debug, Clone)]
struct BootcStatusImage {
    image: BootcStatusImageInfo,
}

#[derive(Deserialize, Debug, Clone)]
struct BootcStatusImageInfo {
    image: BootcStatusImageInfoRef,
}

#[derive(Deserialize, Debug, Clone)]
struct BootcStatusImageInfoRef {
    image: String,
    transport: String,
}

impl BootStatus for BootcStatus {
    fn transaction_in_progress(&self) -> bool {
        // Any call to bootc when a transaction is in progress
        // will cause the process to block effectively making
        // this check useless since bootc will continue with
        // the operation as soon as the current transaction is
        // completed.
        false
    }

    fn booted_image(&self) -> Option<ImageRef<'_>> {
        match self.status.booted.image.image.transport.as_str() {
            "registry" | "containers-storage" => Some(ImageRef::Remote(Cow::Owned(
                Reference::try_from(self.status.booted.image.image.image.as_str())
                    .inspect_err(|e| {
                        warn!(
                            "Failed to parse image ref {}:\n{e}",
                            self.status.booted.image.image.image
                        );
                    })
                    .ok()?,
            ))),
            transport if transport == OCI_ARCHIVE => Some(ImageRef::LocalTar(Cow::Owned(
                PathBuf::from(&self.status.booted.image.image.image),
            ))),
            _ => None,
        }
    }

    fn staged_image(&self) -> Option<ImageRef<'_>> {
        let staged = self.status.staged.as_ref()?;
        match staged.image.image.transport.as_str() {
            "registry" | "containers-storage" => Some(ImageRef::Remote(Cow::Owned(
                Reference::try_from(staged.image.image.image.as_str())
                    .inspect_err(|e| {
                        warn!(
                            "Failed to parse image ref {}:\n{e}",
                            staged.image.image.image
                        );
                    })
                    .ok()?,
            ))),
            transport if transport == OCI_ARCHIVE => Some(ImageRef::LocalTar(Cow::Owned(
                PathBuf::from(&staged.image.image.image),
            ))),
            _ => None,
        }
    }
}
