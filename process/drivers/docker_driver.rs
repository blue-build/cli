use std::{
    ops::Not,
    path::Path,
    process::{Command, ExitStatus},
};

use blue_build_utils::{
    constants::{BLUE_BUILD, DOCKER_HOST, GITHUB_ACTIONS},
    credentials::Credentials,
    get_env_var,
    semver::Version,
    string_vec,
};
use cached::proc_macro::{cached, once};
use colored::Colorize;
use comlexr::{cmd, pipe};
use log::{debug, info, trace, warn};
use miette::{Context, IntoDiagnostic, Result, bail};
use oci_distribution::Reference;
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
        types::{ContainerId, ImageMetadata, ImageRef, Platform},
    },
    logging::CommandLogging,
    signal_handler::{ContainerRuntime, ContainerSignalId, add_cid, remove_cid},
};

use super::opts::{CreateContainerOpts, PruneOpts, RemoveContainerOpts, RemoveImageOpts};

#[derive(Debug, Deserialize)]
struct VerisonJsonClient {
    #[serde(alias = "Version")]
    version: Version,
}

#[derive(Debug, Deserialize)]
struct VersionJson {
    #[serde(alias = "Client")]
    client: VerisonJsonClient,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContextListItem {
    name: String,
}

#[derive(Debug)]
pub struct DockerDriver;

impl DockerDriver {
    fn setup() -> Result<()> {
        #[once(result = true, sync_writes = true)]
        fn exec() -> Result<()> {
            trace!("DockerDriver::setup()");

            if !DockerDriver::has_buildx() {
                bail!("Docker Buildx is required to use the Docker driver");
            }

            let ls_out = {
                let c = cmd!("docker", "buildx", "ls", "--format={{.Name}}");
                trace!("{c:?}");
                c
            }
            .output()
            .into_diagnostic()?;

            if !ls_out.status.success() {
                bail!("{}", String::from_utf8_lossy(&ls_out.stderr));
            }

            let ls_out = String::from_utf8(ls_out.stdout).into_diagnostic()?;

            trace!("{ls_out}");

            if !ls_out.lines().any(|line| line == BLUE_BUILD) {
                let remote = get_env_var(DOCKER_HOST).is_ok();
                if remote {
                    let context_list = get_context_list()?;
                    trace!("{context_list:#?}");

                    if context_list.iter().any(|ctx| ctx.name == BLUE_BUILD).not() {
                        let context_out = {
                            let c = cmd!(
                                "docker",
                                "context",
                                "create",
                                "--from=default",
                                format!("{BLUE_BUILD}0")
                            );
                            trace!("{c:?}");
                            c
                        }
                        .output()
                        .into_diagnostic()?;

                        if context_out.status.success().not() {
                            bail!("{}", String::from_utf8_lossy(&context_out.stderr));
                        }

                        let context_list = get_context_list()?;
                        trace!("{context_list:#?}");
                    }
                }

                let create_out = {
                    let c = cmd!(
                        "docker",
                        "buildx",
                        "create",
                        "--bootstrap",
                        "--driver=docker-container",
                        format!("--name={BLUE_BUILD}"),
                        if remote => format!("{BLUE_BUILD}0"),
                    );
                    trace!("{c:?}");
                    c
                }
                .output()
                .into_diagnostic()?;

                if !create_out.status.success() {
                    bail!("{}", String::from_utf8_lossy(&create_out.stderr));
                }
            }

            Ok(())
        }

        exec()
    }

    #[must_use]
    pub fn has_buildx() -> bool {
        pipe!(cmd!("docker", "--help") | cmd!("grep", "buildx"))
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn get_context_list() -> Result<Vec<ContextListItem>> {
    {
        let c = cmd!("docker", "context", "ls", "--format=json");
        trace!("{c:?}");
        c
    }
    .output()
    .into_diagnostic()
    .and_then(|out| {
        if out.status.success().not() {
            bail!("{}", String::from_utf8_lossy(&out.stderr));
        }
        String::from_utf8(out.stdout).into_diagnostic()
    })?
    .lines()
    .map(|line| serde_json::from_str(line).into_diagnostic())
    .collect()
}

impl DriverVersion for DockerDriver {
    // First docker verison to use buildkit
    // https://docs.docker.com/build/buildkit/
    const VERSION_REQ: &'static str = ">=23";

    fn version() -> Result<Version> {
        trace!("DockerDriver::version()");

        let output = {
            let c = cmd!("docker", "version", "-f", "json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        let version_json: VersionJson = serde_json::from_slice(&output.stdout).into_diagnostic()?;

        Ok(version_json.client.version)
    }
}

impl BuildDriver for DockerDriver {
    fn build(opts: BuildOpts) -> Result<()> {
        trace!("DockerDriver::build({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        let status = {
            let c = cmd!(
                "docker",
                "build",
                if !matches!(opts.platform, Platform::Native) => [
                    "--platform",
                    opts.platform.to_string(),
                ],
                "-t",
                opts.image.to_string(),
                "-f",
                if let Some(cache_from) = opts.cache_from.as_ref() => [
                    "--cache-from",
                    format!(
                        "type=registry,ref={registry}/{repository}",
                        registry = cache_from.registry(),
                        repository = cache_from.repository(),
                    ),
                ],
                if let Some(cache_to) = opts.cache_to.as_ref() => [
                    "--cache-to",
                    format!(
                        "type=registry,ref={registry}/{repository},mode=max",
                        registry = cache_to.registry(),
                        repository = cache_to.repository(),
                    ),
                ],
                opts.containerfile,
                ".",
            );
            trace!("{c:?}");
            c
        }
        .status()
        .into_diagnostic()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(opts: TagOpts) -> Result<()> {
        trace!("DockerDriver::tag({opts:#?})");

        let dest_image_str = opts.dest_image.to_string();

        let status = {
            let c = cmd!("docker", "tag", opts.src_image.to_string(), &dest_image_str);
            trace!("{c:?}");
            c
        }
        .status()
        .into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", dest_image_str.bold().green());
        } else {
            bail!("Failed to tag image {}", dest_image_str.bold().red());
        }
        Ok(())
    }

    fn push(opts: PushOpts) -> Result<()> {
        trace!("DockerDriver::push({opts:#?})");

        let image_str = opts.image.to_string();

        let status = {
            let c = cmd!("docker", "push", &image_str);
            trace!("{c:?}");
            c
        }
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
            let output = pipe!(
                stdin = password;
                {
                    let c = cmd!(
                        "docker",
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
                bail!("Failed to login for docker:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }

        Ok(())
    }

    fn prune(opts: PruneOpts) -> Result<()> {
        trace!("DockerDriver::prune({opts:?})");

        let (system, buildx) = std::thread::scope(
            |scope| -> std::thread::Result<(Result<ExitStatus>, Result<ExitStatus>)> {
                let system = scope.spawn(|| {
                    {
                        let c = cmd!(
                            "docker",
                            "system",
                            "prune",
                            "--force",
                            if opts.all => "--all",
                            if opts.volumes => "--volumes",
                        );
                        trace!("{c:?}");
                        c
                    }
                    .message_status("docker system prune", "Pruning Docker System")
                    .into_diagnostic()
                });

                let buildx = scope.spawn(|| {
                    Self::setup()?;

                    {
                        let c = cmd!(
                            "docker",
                            "buildx",
                            "prune",
                            "--force",
                            format!("--builder={BLUE_BUILD}"),
                            if opts.all => "--all",
                        );
                        trace!("{c:?}");
                        c
                    }
                    .message_status("docker buildx prune", "Pruning Docker Buildx")
                    .into_diagnostic()
                });

                Ok((system.join()?, buildx.join()?))
            },
        )
        .map_err(|e| miette::miette!("{e:?}"))?;

        if !system?.success() {
            bail!("Failed to prune docker system");
        }

        if !buildx?.success() {
            bail!("Failed to prune docker buildx");
        }

        Ok(())
    }

    fn build_tag_push(opts: BuildTagPushOpts) -> Result<Vec<String>> {
        trace!("DockerDriver::build_tag_push({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        Self::setup()?;

        let final_images = get_final_images(opts);

        let first_image = final_images.first().unwrap();

        let status = build_tag_push_cmd(opts, first_image)
            .build_status(first_image, "Building Image")
            .into_diagnostic()?;

        if status.success() {
            if opts.push {
                info!("Successfully built and pushed image {first_image}");
            } else {
                info!("Successfully built image {first_image}");
            }
        } else {
            bail!("Failed to build image {}", first_image);
        }
        Ok(final_images)
    }
}

fn build_tag_push_cmd(opts: BuildTagPushOpts<'_>, first_image: &str) -> Command {
    let c = cmd!(
        "docker",
        "buildx",
        format!("--builder={BLUE_BUILD}"),
        "build",
        ".",
        match &opts.image {
            ImageRef::Remote(_remote) if opts.push => [
                "--output",
                format!(
                    "type=image,name={first_image},push=true,compression={},oci-mediatypes=true",
                    opts.compression
                ),
            ],
            ImageRef::Remote(_remote) if get_env_var(GITHUB_ACTIONS).is_err() => "--load",
            ImageRef::LocalTar(archive_path) => [
                "--output",
                format!("type=oci,dest={}", archive_path.display()),
            ],
            _ => [],
        },
        for opts.image.remote_ref().map_or_else(Vec::new, |image| {
                opts.tags.iter().flat_map(|tag| {
                    vec![
                        "-t".to_string(),
                        format!("{}/{}:{tag}", image.resolve_registry(), image.repository())
                    ]
                }).collect()
            }),
        "--pull",
        if !matches!(opts.platform, Platform::Native) => [
            "--platform",
            opts.platform.to_string(),
        ],
        "-f",
        opts.containerfile,
        if let Some(cache_from) = opts.cache_from.as_ref() => [
            "--cache-from",
            format!(
                "type=registry,ref={cache_from}",
            ),
        ],
        if let Some(cache_to) = opts.cache_to.as_ref() => [
            "--cache-to",
            format!(
                "type=registry,ref={cache_to},mode=max",
            ),
        ],
    );
    trace!("{c:?}");
    c
}

fn get_final_images(opts: BuildTagPushOpts<'_>) -> Vec<String> {
    match &opts.image {
        ImageRef::Remote(image) => {
            if opts.tags.is_empty() {
                let image = image.to_string();
                string_vec![image]
            } else {
                opts.tags
                    .iter()
                    .map(|tag| format!("{}/{}:{tag}", image.resolve_registry(), image.repository()))
                    .collect()
            }
        }
        ImageRef::LocalTar(archive_path) => {
            string_vec![archive_path.display().to_string()]
        }
        ImageRef::Other(other) => {
            string_vec![&**other]
        }
    }
}

impl InspectDriver for DockerDriver {
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata> {
        get_metadata_cache(opts)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ format!("{}-{}", opts.image, opts.platform)}"#,
    sync_writes = "by_key"
)]
fn get_metadata_cache(opts: GetMetadataOpts) -> Result<ImageMetadata> {
    trace!("DockerDriver::get_metadata({opts:#?})");
    let image_str = opts.image.to_string();

    DockerDriver::setup()?;

    let output = {
        let c = cmd!(
            "docker",
            "buildx",
            format!("--builder={BLUE_BUILD}"),
            "imagetools",
            "inspect",
            "--format",
            "{{json .}}",
            &image_str,
        );
        trace!("{c:?}");
        c
    }
    .output()
    .into_diagnostic()?;

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
    fn run(opts: RunOpts) -> Result<ExitStatus> {
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let status = docker_run(opts, &cid_file)
            .build_status(opts.image, "Running container")
            .into_diagnostic()?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: RunOpts) -> Result<std::process::Output> {
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new().into_diagnostic()?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerSignalId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let output = docker_run(opts, &cid_file).output().into_diagnostic()?;

        remove_cid(&cid);

        Ok(output)
    }

    fn create_container(opts: CreateContainerOpts) -> Result<super::types::ContainerId> {
        trace!("DockerDriver::create_container({opts:?})");

        let output = {
            let c = cmd!("docker", "create", opts.image.to_string(), "bash",);
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to create container from image {}", opts.image);
        }

        Ok(ContainerId(
            String::from_utf8(output.stdout.trim_ascii().to_vec()).into_diagnostic()?,
        ))
    }

    fn remove_container(opts: RemoveContainerOpts) -> Result<()> {
        trace!("DockerDriver::remove_container({opts:?})");

        let output = {
            let c = cmd!("docker", "rm", opts.container_id);
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
        trace!("DockerDriver::remove_image({opts:?})");

        let output = {
            let c = cmd!("docker", "rmi", opts.image.to_string());
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

    fn list_images(_privileged: bool) -> Result<Vec<Reference>> {
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "PascalCase")]
        struct Image {
            repository: String,
            tag: String,
        }

        let output = {
            let c = cmd!("docker", "images", "--format", "json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to list images");
        }

        let images: Vec<Image> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| serde_json::from_str::<Image>(line).into_diagnostic())
            .collect::<Result<_>>()?;

        images
            .into_iter()
            .filter(|image| image.repository != "<none>" && image.tag != "<none>")
            .map(|image| {
                format!("{}:{}", image.repository, image.tag)
                    .parse::<Reference>()
                    .into_diagnostic()
                    .with_context(|| format!("While parsing {image:?}"))
            })
            .collect()
    }
}

fn docker_run(opts: RunOpts, cid_file: &Path) -> Command {
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
        opts.image,
        for arg in opts.args.iter() => &**arg,
    );
    trace!("{command:?}");

    command
}
