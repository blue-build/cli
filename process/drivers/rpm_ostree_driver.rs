use std::ops::Not;

use blue_build_utils::constants::OSTREE_UNVERIFIED_IMAGE;
use comlexr::cmd;
use log::trace;
use miette::{Context, IntoDiagnostic, bail};

use crate::logging::CommandLogging;

use super::{BootDriver, BootStatus, opts::SwitchOpts};

mod status;

pub use status::*;

pub struct RpmOstreeDriver;

impl BootDriver for RpmOstreeDriver {
    fn status() -> miette::Result<Box<dyn BootStatus>> {
        let output = {
            let c = cmd!("rpm-ostree", "status", "--json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to get `rpm-ostree` status!");
        }

        trace!("{}", String::from_utf8_lossy(&output.stdout));

        Ok(Box::new(
            serde_json::from_slice::<Status>(&output.stdout)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Failed to deserialize rpm-ostree status:\n{}",
                        String::from_utf8_lossy(&output.stdout)
                    )
                })?,
        ))
    }

    fn switch(opts: SwitchOpts) -> miette::Result<()> {
        let status = {
            let c = cmd!(
                "rpm-ostree",
                "rebase",
                format!("{OSTREE_UNVERIFIED_IMAGE}:containers-storage:{}", opts.image),
                if opts.reboot => "--reboot",
            );

            trace!("{c:?}");
            c
        }
        .build_status(format!("{}", opts.image), "Switching to new image")
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to image {}", opts.image);
        }

        Ok(())
    }

    fn upgrade(opts: SwitchOpts) -> miette::Result<()> {
        let status = {
            let c = cmd!(
                "rpm-ostree",
                "upgrade",
                if opts.reboot => "--reboot",
            );

            trace!("{c:?}");
            c
        }
        .build_status(format!("{}", opts.image), "Switching to new image")
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to image {}", opts.image);
        }

        Ok(())
    }
}
