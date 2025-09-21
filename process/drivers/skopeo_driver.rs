use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};

use super::opts::CopyOciDirOpts;

#[derive(Debug)]
pub struct SkopeoDriver;

impl super::OciCopy for SkopeoDriver {
    fn copy_oci_dir(opts: CopyOciDirOpts) -> Result<()> {
        use crate::logging::CommandLogging;

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
                        concat!(
                            "Password is required to copy ",
                            "OCI directory {dir:?} to remote registry {registry}"
                        ),
                        dir = opts.oci_dir,
                        registry = opts.registry,
                    )
                ],
                if use_sudo => "skopeo",
                "copy",
                opts.oci_dir,
                format!("docker://{}", opts.registry),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.registry.to_string(),
            format!("Copying {} to", opts.oci_dir),
        )
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to copy {} to {}", opts.oci_dir, opts.registry);
        }

        Ok(())
    }
}
