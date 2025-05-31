use std::{borrow::Cow, ops::Not, path::PathBuf};

use blue_build_utils::{
    constants::{OCI_ARCHIVE, SUDO_ASKPASS},
    has_env_var,
};
use comlexr::cmd;
use log::{trace, warn};
use miette::{Context, IntoDiagnostic, Result, bail};
use oci_distribution::Reference;
use serde::Deserialize;

use crate::logging::CommandLogging;

use super::{BootDriver, BootStatus, opts::SwitchOpts, types::ImageRef};

const SUDO_PROMPT: &str = "Password needed to run bootc";

pub struct BootcDriver;

impl BootDriver for BootcDriver {
    fn status() -> Result<Box<dyn BootStatus>> {
        let output = {
            let c = cmd!(
                "sudo",
                if has_env_var(SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    SUDO_PROMPT,
                ],
                "bootc",
                "status",
                "--format=json",
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to get `bootc` status!");
        }

        trace!("{}", String::from_utf8_lossy(&output.stdout));

        Ok(Box::new(
            serde_json::from_slice::<BootcStatus>(&output.stdout)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Failed to deserialize bootc status:\n{}",
                        String::from_utf8_lossy(&output.stdout)
                    )
                })?,
        ))
    }

    fn switch(opts: SwitchOpts) -> Result<()> {
        let status = {
            let c = cmd!(
                "sudo",
                if has_env_var(SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    SUDO_PROMPT,
                ],
                "bootc",
                "switch",
                "--transport=containers-storage",
                opts.image.to_string(),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.image.to_string(),
            format!("Switching to {}", opts.image),
        )
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to {}", opts.image);
        }

        Ok(())
    }

    fn upgrade(opts: SwitchOpts) -> Result<()> {
        let status = {
            let c = cmd!(
                "sudo",
                if has_env_var(SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    SUDO_PROMPT,
                ],
                "bootc",
                "upgrade",
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.image.to_string(),
            format!("Switching to {}", opts.image),
        )
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to {}", opts.image);
        }

        Ok(())
    }
}

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

    fn booted_image(&self) -> Option<ImageRef> {
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

    fn staged_image(&self) -> Option<ImageRef> {
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
