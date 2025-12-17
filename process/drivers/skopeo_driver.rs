use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};

use super::opts::CopyOciOpts;
use crate::logging::CommandLogging;

#[derive(Debug)]
pub struct SkopeoDriver;

impl super::OciCopy for SkopeoDriver {
    fn copy_oci(&self, opts: CopyOciOpts) -> Result<()> {
        trace!("SkopeoDriver::copy_oci({opts:?})");
        let use_sudo = opts.privileged && !blue_build_utils::running_as_root();
        let status = {
            let c = cmd!(
                if use_sudo {
                    "sudo"
                } else {
                    "skopeo"
                },
                if use_sudo && blue_build_utils::has_env_var(blue_build_utils::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    format!(
                        "Password is required to copy {source} to {dest}",
                        source = opts.src_ref,
                        dest = opts.dest_ref,
                    )
                ],
                if use_sudo => "skopeo",
                "copy",
                "--all",
                if opts.retry_count != 0 => format!("--retry-times={}", opts.retry_count),
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
