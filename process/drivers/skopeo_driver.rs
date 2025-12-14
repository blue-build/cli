use blue_build_utils::credentials::Credentials;
use comlexr::{cmd, pipe};
use log::{debug, trace};
use miette::{IntoDiagnostic, Result, bail};

use super::opts::CopyOciSourceOpts;
use crate::logging::CommandLogging;

#[derive(Debug)]
pub struct SkopeoDriver;

impl super::OciCopy for SkopeoDriver {
    fn registry_login(server: &str) -> Result<()> {
        trace!("SkopeoDriver::registry_login()");

        if let Some(Credentials::Basic { username, password }) = Credentials::get(server) {
            let output = pipe!(
                stdin = password.value();
                {
                    let c = cmd!(
                        "skopeo",
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

    fn copy_oci_source(opts: CopyOciSourceOpts) -> Result<()> {
        trace!("SkopeoDriver::copy_oci_source({opts:?})");
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
                        "Password is required to copy {source:?} to remote registry {registry}",
                        source = opts.oci_source,
                        registry = opts.registry,
                    )
                ],
                if use_sudo => "skopeo",
                "copy",
                "--all",
                opts.oci_source.to_os_string(),
                format!("docker://{}", opts.registry),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            opts.registry.to_string(),
            format!("Copying {} to", opts.oci_source),
        )
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to copy {} to {}", opts.oci_source, opts.registry);
        }

        Ok(())
    }
}
