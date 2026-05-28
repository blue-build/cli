use log::trace;
use miette::{IntoDiagnostic, Result, bail};

use super::opts::CopyOciOpts;
use crate::logging::CommandLogging;
use blue_build_utils::{running_as_root, sudo_cmd};

const SUDO_PROMPT: &str = "Password for %u required to run `skopeo copy` as privileged";

#[derive(Debug)]
pub struct SkopeoDriver;

impl super::OciCopy for SkopeoDriver {
    fn copy_oci(&self, opts: CopyOciOpts) -> Result<()> {
        trace!("SkopeoDriver::copy_oci({opts:?})");
        let unshare = opts.podman_unshare && !opts.privileged && !running_as_root();
        let status = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                if unshare { "podman" } else { "skopeo" },
                if unshare => ["unshare", "skopeo"],
                "copy",
                "--all",
                if opts.retry_count != 0 => format!("--retry-times={}", opts.retry_count),
                "--",
                opts.src_ref.to_os_string(),
                opts.dest_ref.to_os_string(),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.dest_ref.to_string(),
            format!("Copying {} to", opts.src_ref),
        )
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to copy {} to {}", opts.src_ref, opts.dest_ref);
        }

        Ok(())
    }
}
