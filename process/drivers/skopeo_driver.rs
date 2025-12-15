use blue_build_utils::credentials::Credentials;
use comlexr::{cmd, pipe};
use log::{debug, trace};
use miette::{IntoDiagnostic, Result, bail};

use super::opts::CopyOciOpts;
use crate::logging::CommandLogging;

#[derive(Debug)]
pub struct SkopeoDriver;

impl super::OciCopy for SkopeoDriver {
    fn registry_login(server: &str) -> Result<()> {
        trace!("SkopeoDriver::registry_login()");

        if let Some(Credentials::Basic { username, password }) = Credentials::get(server)
            && let Ok(skopeo_cmd) = which::which("skopeo")
        {
            let output = pipe!(
                stdin = password.value();
                {
                    let c = cmd!(
                        &skopeo_cmd,
                        "login",
                        "-u",
                        &username,
                        "--password-stdin",
                        server,
                    );
                    trace!("{c:?}");
                    c
                }
            )
            .output()
            .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for skopeo:\n{}", err_out.trim());
            }
            debug!("Logged into {server}");
        }
        Ok(())
    }

    fn copy_oci(opts: CopyOciOpts) -> Result<()> {
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
