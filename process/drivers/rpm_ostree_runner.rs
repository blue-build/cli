use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::Arc,
};

use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};

use crate::{logging::CommandLogging, signal_handler::DetachedContainer};

use super::{
    Driver, OciCopy, PodmanDriver, RunDriver,
    opts::{CopyOciOpts, RunOpts, RunOptsVolume},
};

#[derive(Clone, Debug)]
pub enum RpmOstreeRunner {
    System,
    Container(Arc<RpmOstreeContainer>),
}

#[derive(Debug)]
pub struct RpmOstreeContainer {
    inner: DetachedContainer,
}

impl RpmOstreeRunner {
    /// Initialize a context in which rpm-ostree can be run. If rpm-ostree is installed
    /// on the host, this is essentially a no-op. Otherwise, a container in which rpm-ostree
    /// is installed is started and made ready for execution of further commands.
    ///
    /// # Errors
    /// Returns an error if the container fails to start.
    pub fn start() -> Result<Self> {
        let runner = if which::which("rpm-ostree").is_ok() {
            Self::System
        } else {
            Self::Container(Arc::new(RpmOstreeContainer::start()?))
        };
        Ok(runner)
    }

    /// Produce the arguments to run the given command inside the runner context.
    ///
    /// # Errors
    /// Returns an error if the runner is a container and the container ID file cannot
    /// be read into a string.
    pub fn command_args<T: AsRef<OsStr>, U: AsRef<OsStr>>(
        &self,
        cmd: T,
        args: &[U],
    ) -> Result<(OsString, Vec<OsString>)> {
        if let Self::Container(container) = self {
            container.command_args(cmd, args)
        } else {
            Ok((
                cmd.as_ref().to_owned(),
                args.iter().map(|arg| arg.as_ref().to_owned()).collect(),
            ))
        }
    }

    #[must_use]
    pub fn authfile(&self) -> Option<PathBuf> {
        matches!(self, Self::Container(_)).then(|| PathBuf::from("/run/containers/auth.json"))
    }
}

impl OciCopy for RpmOstreeRunner {
    fn copy_oci(&self, opts: super::opts::CopyOciOpts) -> Result<()> {
        match self {
            Self::System => Driver.copy_oci(opts),
            Self::Container(container) => container.copy_oci(opts),
        }
    }
}

impl RpmOstreeContainer {
    const IMAGE_REF: &str = "ghcr.io/blue-build/rpm-ostree-container:latest";

    fn start() -> Result<Self> {
        let podman_storage_dir = get_podman_info("{{.Store.GraphRoot}}")?;
        let podman_storage_mount = RunOptsVolume::builder()
            .path_or_vol_name(&podman_storage_dir)
            .container_path(&podman_storage_dir)
            .build();
        let runtime_container_dir = get_podman_info("{{.Store.RunRoot}}")?;
        let runtime_container_mount = RunOptsVolume::builder()
            .path_or_vol_name(&runtime_container_dir)
            .container_path("/run/containers")
            .build();

        let container = PodmanDriver::run_detached(
            RunOpts::builder()
                .privileged(true)
                .remove(true)
                .volumes(&[podman_storage_mount, runtime_container_mount])
                .image(Self::IMAGE_REF)
                .args(&[
                    "--storage".to_owned(),
                    podman_storage_dir.clone(),
                    "/bin/sh".to_owned(),
                    "-c".to_owned(),
                    "while true; do sleep 86400; done".to_owned(),
                ])
                .build(),
        )?;

        Ok(Self { inner: container })
    }

    /// Produce the arguments to run the given command inside the container.
    ///
    /// # Errors
    /// Returns an error if the container ID file cannot be read into a string.
    fn command_args<T: AsRef<OsStr>, U: AsRef<OsStr>>(
        &self,
        cmd: T,
        args: &[U],
    ) -> Result<(OsString, Vec<OsString>)> {
        let mut final_args = Vec::with_capacity(args.len() + 3);
        final_args.push(OsString::from("exec"));

        let cid = std::fs::read_to_string(self.inner.cid_path()).into_diagnostic()?;
        final_args.push(cid.into());

        final_args.push(cmd.as_ref().to_owned());
        final_args.extend(args.iter().map(|arg| arg.as_ref().to_owned()));
        Ok((OsString::from("podman"), final_args))
    }
}

impl OciCopy for RpmOstreeContainer {
    fn copy_oci(&self, opts: CopyOciOpts) -> Result<()> {
        trace!("RpmOstreeContainer::copy_oci({opts:?})");
        let use_sudo = opts.privileged && !blue_build_utils::running_as_root();
        let (cmd, args) = self.command_args("skopeo", &["copy"])?;
        let status = {
            let c = cmd!(
                if use_sudo {
                    OsStr::from_bytes(b"sudo")
                } else {
                    &*cmd
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
                if use_sudo => cmd,
                for args,
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

fn get_podman_info(fmt: &str) -> Result<String> {
    let output = cmd!("podman", "info", format!("--format={fmt}"))
        .output()
        .into_diagnostic()?;
    if !output.status.success() {
        bail!("Failed to find podman info {fmt}");
    }
    let mut stdout = output.stdout;
    while stdout.pop_if(|byte| byte.is_ascii_whitespace()).is_some() {}
    String::from_utf8(stdout).into_diagnostic()
}
