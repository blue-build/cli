use std::{
    collections::HashMap,
    env,
    io::Write,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    sync::Mutex,
    time::Duration,
};

use blue_build_utils::{
    cmd,
    constants::{BB_BUILDKIT_CACHE_GHA, CONTAINER_FILE, DOCKER_HOST},
    credentials::Credentials,
    string_vec,
};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, trace, warn};
use miette::{bail, miette, IntoDiagnostic, Report, Result};
use oci_distribution::Reference;
use once_cell::sync::Lazy;
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    drivers::{
        opts::{RunOptsEnv, RunOptsVolume},
        types::{InspectDriverType, Platform},
        Driver,
    },
    logging::{CommandLogging, Logger},
    signal_handler::{add_cid, remove_cid, ContainerId, ContainerRuntime},
};

use super::{
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
    types::ImageMetadata,
    BuildDriver, DriverVersion, InspectDriver, RunDriver,
};

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct DockerImageMetadata {
    config: DockerImageMetadataConfig,
    repo_digests: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct DockerImageMetadataConfig {
    labels: HashMap<String, serde_json::Value>,
}

impl TryFrom<Vec<DockerImageMetadata>> for ImageMetadata {
    type Error = Report;

    fn try_from(mut value: Vec<DockerImageMetadata>) -> Result<Self> {
        if value.is_empty() {
            bail!("Need at least one metadata entry:\n{value:?}");
        }

        let mut value = value.swap_remove(0);
        if value.repo_digests.is_empty() {
            bail!("Metadata requires at least 1 digest:\n{value:#?}");
        }

        let index = value
            .repo_digests
            .iter()
            .enumerate()
            .find(|(_, repo_digest)| verify_image(repo_digest))
            .map(|(index, _)| index)
            .ok_or_else(|| {
                miette!(
                    "No repo digest could be verified:\n{:?}",
                    &value.repo_digests
                )
            })?;

        let digest: Reference = value
            .repo_digests
            .swap_remove(index)
            .parse()
            .into_diagnostic()?;
        let digest = digest
            .digest()
            .ok_or_else(|| miette!("Unable to read digest from {digest}"))?
            .to_string();

        Ok(Self {
            labels: value.config.labels,
            digest,
        })
    }
}

fn verify_image(repo_digest: &str) -> bool {
    let mut command = cmd!("docker", "pull", repo_digest);
    trace!("{command:?}");

    command.output().is_ok_and(|out| out.status.success())
}

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

        trace!("docker tag {} {}", opts.src_image, opts.dest_image);
        let status = cmd!("docker", "tag", &*opts.src_image, &*opts.dest_image,)
            .status()
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(opts: &PushOpts) -> Result<()> {
        trace!("DockerDriver::push({opts:#?})");

        trace!("docker push {}", opts.image);
        let status = cmd!("docker", "push", &*opts.image)
            .status()
            .into_diagnostic()?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image);
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
                stdin = Stdio::piped(),
                stdout = Stdio::piped(),
                stderr = Stdio::piped(),
            );

            trace!("{command:?}");
            let mut child = command.spawn().into_diagnostic()?;

            write!(child.stdin.as_mut().unwrap(), "{password}").into_diagnostic()?;

            let output = child.wait_with_output().into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
        }

        Ok(())
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<Vec<String>> {
        trace!("DockerDriver::build_tag_push({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        let mut command = cmd!(
            "docker",
            "buildx",
            |command|? {
                if !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty()) {
                    Self::setup()?;
                    cmd!(command, "--builder=bluebuild");
                }
            },
            "build",
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

        let final_images = match (opts.image.as_deref(), opts.archive_path.as_deref()) {
            (Some(image), None) => {
                let images = if opts.tags.is_empty() {
                    cmd!(command, "-t", image);
                    string_vec![image]
                } else {
                    opts.tags.iter().for_each(|tag| {
                        cmd!(command, "-t", format!("{image}:{tag}"));
                    });
                    opts.tags
                        .iter()
                        .map(|tag| format!("{image}:{tag}"))
                        .collect()
                };
                let first_image = images.first().unwrap();

                if opts.push {
                    cmd!(
                        command,
                        "--output",
                        format!(
                            "type=image,name={first_image},push=true,compression={},oci-mediatypes=true",
                            opts.compression
                        ),
                        // Load the image to the local image registry
                        // if the inspect driver is docker so that
                        // we don't have to pull the image again to inspect.
                        if matches!(
                            Driver::get_inspect_driver(),
                            InspectDriverType::Docker,
                        ) => "--load",
                    );
                } else {
                    cmd!(command, "--load");
                }
                images
            }
            (None, Some(archive_path)) => {
                cmd!(command, "--output", format!("type=oci,dest={archive_path}"));
                string_vec![archive_path]
            }
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        };
        let display_image = final_images.first().unwrap(); // There will always be at least one image

        cmd!(command, ".");

        trace!("{command:?}");
        if command
            .status_image_ref_progress(display_image, "Building Image")
            .into_diagnostic()?
            .success()
        {
            if opts.push {
                info!("Successfully built and pushed image {}", display_image);
            } else {
                info!("Successfully built image {}", display_image);
            }
        } else {
            bail!("Failed to build image {}", display_image);
        }
        Ok(final_images)
    }
}

impl InspectDriver for DockerDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("DockerDriver::get_metadata({opts:#?})");

        let url = opts.tag.as_ref().map_or_else(
            || format!("{}", opts.image),
            |tag| format!("{}:{tag}", opts.image),
        );

        let progress = Logger::multi_progress().add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner())
                .with_message(format!(
                    "Inspecting metadata for {}, pulling image...",
                    url.bold()
                )),
        );
        progress.enable_steady_tick(Duration::from_millis(100));

        let mut command = cmd!(
            "docker",
            "pull",
            if !matches!(opts.platform, Platform::Native) => [
                "--platform",
                opts.platform.to_string(),
            ],
            &url,
        );
        trace!("{command:?}");

        let output = command.output().into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to pull {} for inspection!", url.bold());
        }

        let mut command = cmd!("docker", "image", "inspect", "--format=json", &url);
        trace!("{command:?}");

        let output = command.output().into_diagnostic()?;

        progress.finish_and_clear();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            info!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }

        serde_json::from_slice::<Vec<DockerImageMetadata>>(&output.stdout)
            .into_diagnostic()
            .inspect(|metadata| trace!("{metadata:#?}"))
            .and_then(ImageMetadata::try_from)
            .inspect(|metadata| trace!("{metadata:#?}"))
    }
}

impl RunDriver for DockerDriver {
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus> {
        let cid_path = TempDir::new("docker")?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let status = docker_run(opts, &cid_file)
            .status_image_ref_progress(&*opts.image, "Running container")?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: &RunOpts) -> std::io::Result<std::process::Output> {
        let cid_path = TempDir::new("docker")?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let output = docker_run(opts, &cid_file).output()?;

        remove_cid(&cid);

        Ok(output)
    }
}

fn docker_run(opts: &RunOpts, cid_file: &Path) -> Command {
    let command = cmd!(
        "docker",
        "run",
        format!("--cidfile={}", cid_file.display()),
        if opts.privileged => "--privileged",
        if opts.remove => "--rm",
        if opts.pull => "--pull=always",
        for RunOptsVolume { path_or_vol_name, container_path } in opts.volumes.iter() => [
            "--volume",
            format!("{path_or_vol_name}:{container_path}"),
        ],
        for RunOptsEnv { key, value } in opts.env_vars.iter() => [
            "--env",
            format!("{key}={value}"),
        ],
        |command| {
            match (opts.uid, opts.gid) {
                (Some(uid), None) => cmd!(command, "-u", format!("{uid}")),
                (Some(uid), Some(gid)) => cmd!(command, "-u", format!("{}:{}", uid, gid)),
                _ => {}
            }
        },
        &*opts.image,
        for arg in opts.args.iter() => &**arg,
    );
    trace!("{command:?}");

    command
}
