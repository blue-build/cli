use std::{
    ffi::{OsStr, OsString},
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use blue_build_utils::{container::Digest, tempdir};
use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};
use tempfile::TempDir;

use crate::{logging::CommandLogging, run_volumes, signal_handler::DetachedContainer};

use super::{
    Driver, OciCopy, PodmanDriver, RunDriver,
    opts::{CopyOciOpts, RunOpts},
};

#[derive(Debug)]
pub enum RpmOstreeRunner {
    System(TempDir),
    Container(Arc<RpmOstreeContainer>),
}

#[derive(Debug)]
pub struct RpmOstreeContainer {
    inner: DetachedContainer,
    tmp_dir: TempDir,
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
            Self::System(tempdir()?)
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
    ) -> (OsString, Vec<OsString>) {
        if let Self::Container(container) = self {
            container.command_args(cmd, args)
        } else {
            (
                cmd.as_ref().to_owned(),
                args.iter().map(|arg| arg.as_ref().to_owned()).collect(),
            )
        }
    }

    #[must_use]
    pub fn authfile(&self) -> Option<PathBuf> {
        matches!(self, Self::Container(_)).then(|| PathBuf::from("/run/containers/auth.json"))
    }

    #[must_use]
    pub fn tmp_dir(&self) -> &Path {
        match self {
            Self::System(tmp_dir) => tmp_dir.path(),
            Self::Container(container) => container.tmp_dir.path(),
        }
    }
}

impl OciCopy for RpmOstreeRunner {
    fn copy_oci(&self, opts: super::opts::CopyOciOpts) -> Result<Digest> {
        match self {
            Self::System(_) => Driver.copy_oci(opts),
            Self::Container(container) => container.copy_oci(opts),
        }
    }
}

impl RpmOstreeContainer {
    const IMAGE_REF: &str = "ghcr.io/blue-build/rpm-ostree-container:latest@sha256:219535fd1e526c5eead25e43103348ca5b2e0677679316a1f04af60c2ea003eb";

    fn start() -> Result<Self> {
        let podman_storage_dir = PodmanDriver::get_podman_info("{{.Store.GraphRoot}}")?;
        let runtime_container_dir = PodmanDriver::get_podman_info("{{.Store.RunRoot}}")?;
        let tmp_dir = tempdir()?;
        let tmp_dir_path = tmp_dir.path().display().to_string();

        let inner = PodmanDriver::run_detached(
            RunOpts::builder()
                .pull(true)
                .privileged(true)
                .rootless(true)
                .remove(true)
                .volumes(&run_volumes![
                    &podman_storage_dir => &podman_storage_dir,
                    &runtime_container_dir => "/run/containers",
                    &tmp_dir_path => &tmp_dir_path,
                ])
                .image(Self::IMAGE_REF)
                .args(&bon::vec![
                    "--storage",
                    &podman_storage_dir,
                    "/bin/sh",
                    "-c",
                    "while true; do sleep 86400; done",
                ])
                .build(),
        )?;

        Ok(Self { inner, tmp_dir })
    }

    /// Produce the arguments to run the given command inside the container.
    ///
    /// # Errors
    /// Returns an error if the container ID file cannot be read into a string.
    fn command_args<T: AsRef<OsStr>, U: AsRef<OsStr>>(
        &self,
        cmd: T,
        args: &[U],
    ) -> (OsString, Vec<OsString>) {
        let mut final_args = Vec::with_capacity(args.len() + 3);
        final_args.push(OsString::from("exec"));
        final_args.push(self.inner.id().into());
        final_args.push(cmd.as_ref().to_owned());
        final_args.extend(args.iter().map(|arg| arg.as_ref().to_owned()));
        (OsString::from("podman"), final_args)
    }
}

impl OciCopy for RpmOstreeContainer {
    fn copy_oci(&self, opts: CopyOciOpts) -> Result<Digest> {
        trace!("RpmOstreeContainer::copy_oci({opts:?})");
        let use_sudo = opts.privileged && !blue_build_utils::running_as_root();
        let (cmd, args) = self.command_args("skopeo", &["copy"]);

        let digestfile = self.tmp_dir.path().join("digest");

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
                format!("--digestfile={}", digestfile.display()),
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

        fs::read_to_string(digestfile).into_diagnostic()?.parse()
    }
}
