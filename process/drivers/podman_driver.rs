use std::{
    ops::Not,
    path::Path,
    process::{Command, ExitStatus},
};

use blue_build_utils::{
    constants::USER, credentials::Credentials, get_env_var, secret::SecretArgs, semver::Version,
    sudo_cmd,
};
use colored::Colorize;
use comlexr::{cmd, pipe};
use log::{debug, error, info, trace};
use miette::{Context, IntoDiagnostic, Result, bail};
use oci_distribution::Reference;
use serde::Deserialize;
use tempfile::TempDir;

use super::{
    ContainerMountDriver, RechunkDriver,
    opts::{
        ContainerOpts, CreateContainerOpts, PruneOpts, RemoveContainerOpts, RemoveImageOpts,
        VolumeOpts,
    },
    types::{ContainerId, MountId},
};
use crate::{
    drivers::{
        BuildDriver, DriverVersion, RunDriver,
        opts::{BuildOpts, PushOpts, RunOpts, RunOptsEnv, RunOptsVolume, TagOpts},
    },
    logging::CommandLogging,
    signal_handler::{ContainerRuntime, ContainerSignalId, add_cid, remove_cid},
};

const SUDO_PROMPT: &str = "Password for %u required to run 'podman' as privileged";

#[derive(Debug, Deserialize)]
struct PodmanVersionJsonClient {
    #[serde(alias = "Version")]
    pub version: Version,
}

#[derive(Debug, Deserialize)]
struct PodmanVersionJson {
    #[serde(alias = "Client")]
    pub client: PodmanVersionJsonClient,
}

#[derive(Debug)]
pub struct PodmanDriver;

impl PodmanDriver {
    /// Copy an image from the user container
    /// store to the root container store for
    /// booting off of.
    ///
    /// # Errors
    /// Will error if the image can't be copied.
    pub fn copy_image_to_root_store(image: &Reference) -> Result<()> {
        let image = image.whole();
        let status = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                "podman",
                "image",
                "scp",
                format!("{}@localhost::{image}", get_env_var(USER)?),
                "root@localhost::"
            );
            trace!("{c:?}");
            c
        }
        .build_status(&image, "Copying image to root container store")
        // .status()
        .into_diagnostic()?;

        if status.success().not() {
            bail!(
                "Failed to copy image {} to root container store",
                image.bold()
            );
        }

        Ok(())
    }
}

impl DriverVersion for PodmanDriver {
    // First podman version to use buildah v1.24
    // https://github.com/containers/podman/blob/main/RELEASE_NOTES.md#400
    const VERSION_REQ: &'static str = ">=4";

    fn version() -> Result<Version> {
        trace!("PodmanDriver::version()");

        let output = {
            let c = cmd!("podman", "version", "-f", "json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        let version_json: PodmanVersionJson = serde_json::from_slice(&output.stdout)
            .inspect_err(|e| error!("{e}: {}", String::from_utf8_lossy(&output.stdout)))
            .into_diagnostic()?;
        trace!("{version_json:#?}");

        Ok(version_json.client.version)
    }
}

impl BuildDriver for PodmanDriver {
    fn build(opts: BuildOpts) -> Result<()> {
        trace!("PodmanDriver::build({opts:#?})");

        let temp_dir = TempDir::new()
            .into_diagnostic()
            .wrap_err("Failed to create temporary directory for secrets")?;

        let command = sudo_cmd!(
            prompt = SUDO_PROMPT,
            sudo_check = opts.privileged,
            "podman",
            "build",
            if let Some(platform) = opts.platform => [
                "--platform",
                platform.to_string(),
            ],
            if let Some(cache_from) = opts.cache_from.as_ref() => [
                "--cache-from",
                format!(
                    "{}/{}",
                    cache_from.registry(),
                    cache_from.repository()
                ),
            ],
            if let Some(cache_to) = opts.cache_to.as_ref() => [
                "--cache-to",
                format!(
                    "{}/{}",
                    cache_to.registry(),
                    cache_to.repository()
                ),
            ],
            "--pull=true",
            if opts.host_network => "--net=host",
            format!("--layers={}", !opts.squash),
            "-f",
            opts.containerfile,
            "-t",
            opts.image.to_string(),
            for opts.secrets.args(&temp_dir)?,
            if opts.secrets.ssh() => "--ssh",
            ".",
        );

        trace!("{command:?}");
        let status = command
            .build_status(opts.image.to_string(), "Building Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(opts: TagOpts) -> Result<()> {
        trace!("PodmanDriver::tag({opts:#?})");

        let dest_image_str = opts.dest_image.to_string();

        let mut command = sudo_cmd!(
            prompt = SUDO_PROMPT,
            sudo_check = opts.privileged,
            "podman",
            "tag",
            opts.src_image.to_string(),
            &dest_image_str
        );

        trace!("{command:?}");
        let status = command.status().into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", dest_image_str.bold().green());
        } else {
            bail!("Failed to tag image {}", dest_image_str.bold().red());
        }
        Ok(())
    }

    fn push(opts: PushOpts) -> Result<()> {
        trace!("PodmanDriver::push({opts:#?})");

        let image_str = opts.image.to_string();

        let command = sudo_cmd!(
            prompt = SUDO_PROMPT,
            sudo_check = opts.privileged,
            "podman",
            "push",
            format!(
                "--compression-format={}",
                opts.compression_type.unwrap_or_default()
            ),
            &image_str,
        );

        trace!("{command:?}");
        let status = command
            .build_status(&image_str, "Pushing Image")
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", image_str.bold().green());
        } else {
            bail!("Failed to push image {}", image_str.bold().red());
        }
        Ok(())
    }

    fn login() -> Result<()> {
        trace!("PodmanDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            let output = pipe!(
                stdin = password;
                {
                    let c = cmd!(
                        "podman",
                        "login",
                        "-u",
                        username,
                        "--password-stdin",
                        registry,
                    );
                    trace!("{c:?}");
                    c
                }
            )
            .output()
            .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for podman:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }
        Ok(())
    }

    fn prune(opts: PruneOpts) -> Result<()> {
        trace!("PodmanDriver::prune({opts:?})");

        let status = {
            let c = cmd!(
                "podman",
                "system",
                "prune",
                "--force",
                if opts.all => "--all",
                if opts.volumes => "--volumes",
            );
            trace!("{c:?}");
            c
        }
        .message_status("podman system prune", "Pruning Podman System")
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to prune podman");
        }

        Ok(())
    }
}

impl ContainerMountDriver for PodmanDriver {
    fn mount_container(opts: ContainerOpts) -> Result<MountId> {
        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "mount",
                opts.container_id,
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to mount container {}", opts.container_id);
        }

        Ok(MountId(
            String::from_utf8(output.stdout.trim_ascii().to_vec()).into_diagnostic()?,
        ))
    }

    fn unmount_container(opts: ContainerOpts) -> Result<()> {
        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "unmount",
                opts.container_id
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to unmount container {}", opts.container_id);
        }

        Ok(())
    }

    fn remove_volume(opts: VolumeOpts) -> Result<()> {
        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "volume",
                "rm",
                opts.volume_id
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to remove volume {}", &opts.volume_id);
        }

        Ok(())
    }
}

impl RechunkDriver for PodmanDriver {}

impl RunDriver for PodmanDriver {
    fn run(opts: RunOpts) -> Result<ExitStatus> {
        trace!("PodmanDriver::run({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");

        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Podman, opts.privileged);

        add_cid(&cid);

        let status = podman_run(opts, &cid_file)
            .build_status(opts.image, "Running container")
            .into_diagnostic()?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: RunOpts) -> Result<std::process::Output> {
        trace!("PodmanDriver::run_output({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");

        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Podman, opts.privileged);

        add_cid(&cid);

        let output = podman_run(opts, &cid_file).output().into_diagnostic()?;

        remove_cid(&cid);

        Ok(output)
    }

    fn create_container(opts: CreateContainerOpts) -> Result<ContainerId> {
        trace!("PodmanDriver::create_container({opts:?})");

        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "create",
                opts.image.to_string(),
                "bash"
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to create a container from image {}", opts.image);
        }

        Ok(ContainerId(
            String::from_utf8(output.stdout.trim_ascii().to_vec()).into_diagnostic()?,
        ))
    }

    fn remove_container(opts: RemoveContainerOpts) -> Result<()> {
        trace!("PodmanDriver::remove_container({opts:?})");

        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "rm",
                opts.container_id,
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to remove container {}", opts.container_id);
        }

        Ok(())
    }

    fn remove_image(opts: RemoveImageOpts) -> Result<()> {
        trace!("PodmanDriver::remove_image({opts:?})");

        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = opts.privileged,
                "podman",
                "rmi",
                opts.image.to_string()
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to remove the image {}", opts.image);
        }

        Ok(())
    }

    fn list_images(privileged: bool) -> Result<Vec<Reference>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Image {
            names: Option<Vec<String>>,
        }

        trace!("PodmanDriver::list_images({privileged})");

        let output = {
            let c = sudo_cmd!(
                prompt = SUDO_PROMPT,
                sudo_check = privileged,
                "podman",
                "images",
                "--format",
                "json"
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to list images");
        }

        let images: Vec<Image> = serde_json::from_slice(&output.stdout).into_diagnostic()?;

        images
            .into_iter()
            .filter_map(|image| image.names)
            .flat_map(|names| {
                names
                    .into_iter()
                    .map(|name| name.parse::<Reference>().into_diagnostic())
            })
            .collect()
    }
}

fn podman_run(opts: RunOpts, cid_file: &Path) -> Command {
    let command = sudo_cmd!(
        prompt = SUDO_PROMPT,
        sudo_check = opts.privileged,
        "podman",
        "run",
        format!("--cidfile={}", cid_file.display()),
        if opts.privileged => [
            "--privileged",
            "--network=host",
        ],
        if opts.remove => "--rm",
        if opts.pull => "--pull=always",
        if let Some(user) = opts.user.as_ref() => format!("--user={user}"),
        for RunOptsVolume { path_or_vol_name, container_path } in opts.volumes.iter() => [
            "--volume",
            format!("{path_or_vol_name}:{container_path}"),
        ],
        for RunOptsEnv { key, value } in opts.env_vars.iter() => [
            "--env",
            format!("{key}={value}"),
        ],
        opts.image,
        for arg in opts.args.iter() => &**arg,
    );
    trace!("{command:?}");

    command
}
