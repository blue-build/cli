use std::ops::Not;

use blue_build_utils::sudo_cmd;
use log::trace;
use miette::{Context, IntoDiagnostic, Result, bail};

use crate::logging::CommandLogging;

use super::{BootDriver, BootStatus, opts::SwitchOpts};

mod status;

pub use status::*;

const SUDO_PROMPT: &str = "Password needed to run bootc";

pub struct BootcDriver;

impl BootDriver for BootcDriver {
    fn status() -> Result<Box<dyn BootStatus>> {
        let output = {
            let c = sudo_cmd!(prompt = SUDO_PROMPT, "bootc", "status", "--format=json");
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
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
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
            let c = sudo_cmd!(prompt = SUDO_PROMPT, "bootc", "upgrade");
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
