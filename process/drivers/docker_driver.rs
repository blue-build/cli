use std::{
    env,
    io::Write,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::Mutex,
};

use blue_build_utils::{
    constants::{BB_BUILDKIT_CACHE_GHA, CONTAINER_FILE, DOCKER_HOST, GITHUB_ACTIONS},
    credentials::Credentials,
    string_vec,
};
use cached::proc_macro::cached;
use colored::Colorize;
use comlexr::cmd;
use log::{debug, info, trace, warn};
use miette::{bail, miette, IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use semver::Version;
use serde::Deserialize;
use tempfile::TempDir;

mod metadata;

use crate::{
    drivers::{
        opts::{
            BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, RunOptsEnv,
            RunOptsVolume, TagOpts,
        },
        traits::{BuildDriver, DriverVersion, InspectDriver, RunDriver},
        types::ImageMetadata,
        types::Platform,
    },
    logging::CommandLogging,
    signal_handler::{add_cid, remove_cid, ContainerRuntime, ContainerSignalId},
};

#[derive(Debug, Deserialize)]
struct DockerVerisonJsonClient {
    #[serde(alias = "Version")]
    pub version: Version,
}

#[derive(Debug, Deserialize)]
struct DockerVersionJson {
    #[serde(alias = "Client")]
    pub client: DockerVerisonJsonClient,
}

static DOCKER_SETUP: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

#[derive(Debug)]
pub struct DockerDriver;

impl DockerDriver {
    fn setup() -> Result<()> {
        trace!("DockerDriver::setup()");

        let mut lock = DOCKER_SETUP.lock().expect("Should lock");

        if *lock {
            drop(lock);
            return Ok(());
        }

        trace!("docker buildx ls --format={}", "{{.Name}}");
        let ls_out = cmd!("docker", "buildx", "ls", "--format={{.Name}}")
            .output()
            .into_diagnostic()?;

        if !ls_out.status.success() {
            bail!("{}", String::from_utf8_lossy(&ls_out.stderr));
        }

        let ls_out = String::from_utf8(ls_out.stdout).into_diagnostic()?;

        trace!("{ls_out}");

        if !ls_out.lines().any(|line| line == "bluebuild") {
            trace!("docker buildx create --bootstrap --driver=docker-container --name=bluebuild");
            let create_out = cmd!(
                "docker",
                "buildx",
                "create",
                "--bootstrap",
                "--driver=docker-container",
                "--name=bluebuild",
            )
            .output()
            .into_diagnostic()?;

            if !create_out.status.success() {
                bail!("{}", String::from_utf8_lossy(&create_out.stderr));
            }
        }

        *lock = true;
        drop(lock);
        Ok(())
    }
}

impl DriverVersion for DockerDriver {
    // First docker verison to use buildkit
    // https://docs.docker.com/build/buildkit/
    const VERSION_REQ: &'static str = ">=23";

    fn version() -> Result<Version> {
        let output = cmd!("docker", "version", "-f", "json")
            .output()
            .into_diagnostic()?;

        let version_json: DockerVersionJson =
            serde_json::from_slice(&output.stdout).into_diagnostic()?;

        Ok(version_json.client.version)
    }
}

impl BuildDriver for DockerDriver {
    fn build(opts: &BuildOpts) -> Result<()> {
        trace!("DockerDriver::build({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        trace!("docker build -t {} -f {CONTAINER_FILE} .", opts.image);
        let status = cmd!(
            "docker",
            "build",
            if !matches!(opts.platform, Platform::Native) => [
                "--platform",
                opts.platform.to_string(),
            ],
            "-t",
            &*opts.image,
            "-f",
            &*opts.containerfile,
            ".",
        )
        .status()
        .into_diagnostic()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(opts: &TagOpts) -> Result<()> {
        trace!("DockerDriver::tag({opts:#?})");

        let dest_image_str = opts.dest_image.to_string();

        trace!("docker tag {} {}", opts.src_image, opts.dest_image);
        let status = cmd!("docker", "tag", opts.src_image.to_string(), &dest_image_str)
            .status()
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", dest_image_str.bold().green());
        } else {
            bail!("Failed to tag image {}", dest_image_str.bold().red());
        }
        Ok(())
    }

    fn push(opts: &PushOpts) -> Result<()> {
        trace!("DockerDriver::push({opts:#?})");

        let image_str = opts.image.to_string();

        trace!("docker push {}", opts.image);
        let status = cmd!("docker", "push", &image_str)
            .status()
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", image_str.bold().green());
        } else {
            bail!("Failed to push image {}", image_str.bold().red());
        }
        Ok(())
    }

    fn login() -> Result<()> {
        trace!("DockerDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            let mut command = cmd!(
                "docker",
                "login",
                "-u",
                username,
                "--password-stdin",
                registry,
            );
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            trace!("{command:?}");
            let mut child = command.spawn().into_diagnostic()?;

            write!(
                child
                    .stdin
                    .as_mut()
                    .ok_or_else(|| miette!("Unable to open pipe to stdin"))?,
                "{password}"
            )
            .into_diagnostic()?;

            let output = child.wait_with_output().into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }

        Ok(())
    }

    #[cfg(feature = "prune")]
    fn prune(opts: &super::opts::PruneOpts) -> Result<()> {
        trace!("DockerDriver::prune({opts:?})");

        let (system, buildx) = std::thread::scope(
            |scope| -> std::thread::Result<(Result<ExitStatus>, Result<ExitStatus>)> {
                let system = scope.spawn(|| {
                    cmd!(
                        "docker",
                        "system",
                        "prune",
                        "--force",
                        if opts.all => "--all",
                        if opts.volumes => "--volumes",
                    )
                    .message_status("docker system prune", "Pruning Docker System")
                    .into_diagnostic()
                });

                let buildx = scope.spawn(|| {
                    let run_setup = !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty());

                    if run_setup {
                        Self::setup()?;
                    }

                    cmd!(
                        "docker",
                        "buildx",
                        "prune",
                        "--force",
                        if run_setup => "--builder=bluebuild",
                        if opts.all => "--all",
                    )
                    .message_status("docker buildx prune", "Pruning Docker Buildx")
                    .into_diagnostic()
                });

                Ok((system.join()?, buildx.join()?))
            },
        )
        .map_err(|e| miette!("{e:?}"))?;

        if !system?.success() {
            bail!("Failed to prune docker system");
        }

        if !buildx?.success() {
            bail!("Failed to prune docker buildx");
        }

        Ok(())
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<Vec<String>> {
        trace!("DockerDriver::build_tag_push({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        let run_setup = !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty());

        if run_setup {
            Self::setup()?;
        }

        let final_images = match (opts.image, opts.archive_path.as_deref()) {
            (Some(image), None) => {
                let images = if opts.tags.is_empty() {
                    let image = image.to_string();
                    string_vec![image]
                } else {
                    opts.tags
                        .iter()
                        .map(|tag| {
                            format!("{}/{}:{tag}", image.resolve_registry(), image.repository())
                        })
                        .collect()
                };

                images
            }
            (None, Some(archive_path)) => {
                string_vec![archive_path.display().to_string()]
            }
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        };

        let first_image = final_images.first().unwrap();

        let command = cmd!(
            "docker",
            "buildx",
            if run_setup => "--builder=bluebuild",
            "build",
            ".",
            match (opts.image, opts.archive_path.as_deref()) {
                (Some(_), None) if opts.push => [
                    "--output",
                    format!(
                        "type=image,name={first_image},push=true,compression={},oci-mediatypes=true",
                        opts.compression
                    ),
                ],
                (Some(_), None) if env::var(GITHUB_ACTIONS).is_err() => "--load",
                (None, Some(archive_path)) => [
                    "--output",
                    format!("type=oci,dest={}", archive_path.display()),
                ],
                _ => [],
            },
            "--pull",
            if !matches!(opts.platform, Platform::Native) => [
                "--platform",
                opts.platform.to_string(),
            ],
            "-f",
            &*opts.containerfile,
            // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
            if env::var(BB_BUILDKIT_CACHE_GHA)
                .map_or_else(|_| false, |e| e == "true") => [
                    "--cache-from",
                    "type=gha",
                    "--cache-to",
                    "type=gha",
                ],
        );

        trace!("{command:?}");
        if command
            .build_status(first_image, "Building Image")
            .into_diagnostic()?
            .success()
        {
            if opts.push {
                info!("Successfully built and pushed image {}", first_image);
            } else {
                info!("Successfully built image {}", first_image);
            }
        } else {
            bail!("Failed to build image {}", first_image);
        }
        Ok(final_images)
    }
}

impl InspectDriver for DockerDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        get_metadata_cache(opts)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ format!("{}-{}", opts.image, opts.platform)}"#,
    sync_writes = true
)]
fn get_metadata_cache(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
    trace!("DockerDriver::get_metadata({opts:#?})");
    let image_str = opts.image.to_string();

    let run_setup = !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty());

    if run_setup {
        DockerDriver::setup()?;
    }

    let mut command = cmd!(
        "docker",
        "buildx",
        if run_setup => "--builder=bluebuild",
        "imagetools",
        "inspect",
        "--format",
        "{{json .}}",
        &image_str,
    );
    trace!("{command:?}");

    let output = command.output().into_diagnostic()?;

    if output.status.success() {
        info!("Successfully inspected image {}!", image_str.bold().green());
    } else {
        bail!("Failed to inspect image {}", image_str.bold().red())
    }

    serde_json::from_slice::<metadata::Metadata>(&output.stdout)
        .into_diagnostic()
        .inspect(|metadata| trace!("{metadata:#?}"))
        .and_then(|metadata| ImageMetadata::try_from((metadata, opts.platform)))
        .inspect(|metadata| trace!("{metadata:#?}"))
}

impl RunDriver for DockerDriver {
    fn run(opts: &RunOpts) -> Result<ExitStatus> {
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let status = docker_run(opts, &cid_file)
            .build_status(&*opts.image, "Running container")
            .into_diagnostic()?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: &RunOpts) -> Result<std::process::Output> {
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let output = docker_run(opts, &cid_file).output().into_diagnostic()?;

        remove_cid(&cid);

        Ok(output)
    }
}

fn docker_run(opts: &RunOpts, cid_file: &Path) -> Command {
    let command = cmd!(
        "docker",
        "run",
        "--cidfile",
        cid_file,
        if opts.privileged => "--privileged",
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
        &*opts.image,
        for arg in opts.args.iter() => &**arg,
    );
    trace!("{command:?}");

    command
}
