use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStringExt,
    path::PathBuf,
    process::Stdio,
};

use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};

#[derive(Clone, Debug)]
pub enum RpmOstreeRunner {
    System,
    Container(RpmOstreeContainer),
}

#[derive(Clone, Debug)]
pub struct RpmOstreeContainer {
    id: OsString,
}

impl RpmOstreeRunner {
    pub fn start() -> Result<Self> {
        let runner = if which::which("rpm-ostree").is_ok() {
            Self::System
        } else {
            Self::Container(RpmOstreeContainer::start()?)
        };
        Ok(runner)
    }

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

    pub fn authfile(&self) -> Option<PathBuf> {
        matches!(self, Self::Container(_)).then(|| PathBuf::from("/run/containers/auth.json"))
    }
}

impl RpmOstreeContainer {
    const IMAGE_REF: &str = "ghcr.io/blue-build/rpm-ostree-container:latest";

    fn start() -> Result<Self> {
        let podman_storage_dir = get_podman_info("{{.Store.GraphRoot}}")?;
        let podman_storage_mount = {
            let mut out = podman_storage_dir.clone().into_vec();
            out.push(b':');
            out.extend_from_within(0..podman_storage_dir.len());
            OsString::from_vec(out)
        };
        let runtime_container_mount = {
            let mut out = get_podman_info("{{.Store.RunRoot}}")?.into_vec();
            out.extend_from_slice(b":/run/containers");
            OsString::from_vec(out)
        };

        let output = cmd!(
            "podman",
            "run",
            "--detach",
            "--privileged",
            "--rm",
            "-v",
            podman_storage_mount,
            "-v",
            runtime_container_mount,
            Self::IMAGE_REF,
            "--storage",
            podman_storage_dir,
            "/bin/sh",
            "-c",
            "while true; do sleep 86400; done",
        )
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!(
                "Failed to start image {}\nstderr: {}",
                Self::IMAGE_REF,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let mut stdout = output.stdout;
        while stdout.pop_if(|byte| byte.is_ascii_whitespace()).is_some() {}
        let container_id = OsString::from_vec(stdout);
        trace!(
            "Started RpmOstreeContainer with ID: {}",
            container_id.display()
        );
        Ok(Self { id: container_id })
    }

    fn command_args<T: AsRef<OsStr>, U: AsRef<OsStr>>(
        &self,
        cmd: T,
        args: &[U],
    ) -> (OsString, Vec<OsString>) {
        let mut final_args = Vec::with_capacity(args.len() + 3);
        final_args.push(OsString::from("exec"));
        final_args.push(self.id.clone());
        final_args.push(cmd.as_ref().to_owned());
        final_args.extend(args.iter().map(|arg| arg.as_ref().to_owned()));
        (OsString::from("podman"), final_args)
    }
}

impl Drop for RpmOstreeContainer {
    fn drop(&mut self) {
        let _ = cmd!("podman", "stop", &self.id)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .and_then(|mut child| child.wait());
    }
}

fn get_podman_info(fmt: &str) -> Result<OsString> {
    let output = cmd!("podman", "info", format!("--format={fmt}"))
        .output()
        .into_diagnostic()?;
    if !output.status.success() {
        bail!("Failed to find podman info {fmt}");
    }
    let mut stdout = output.stdout;
    while stdout.pop_if(|byte| byte.is_ascii_whitespace()).is_some() {}
    Ok(OsString::from_vec(stdout))
}
