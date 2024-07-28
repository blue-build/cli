use std::{
    env, fs,
    path::Path,
    process::{Command, ExitStatus},
    sync::Mutex,
    time::Duration,
};

use blue_build_utils::{
    cmd,
    constants::{
        BB_BUILDKIT_CACHE_GHA, CONTAINER_FILE, COSIGN_IMAGE, COSIGN_PASSWORD, COSIGN_PRIVATE_KEY,
        COSIGN_PUB_PATH, COSIGN_YES, DOCKER_HOST, GITHUB_TOKEN, SIGSTORE_ID_TOKEN, SKOPEO_IMAGE,
    },
    string, string_vec,
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, trace, warn};
use miette::{bail, Context, IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    credentials::Credentials,
    drivers::{image_metadata::ImageMetadata, GROUP, USER},
    logging::{CommandLogging, Logger},
    run_envs, run_volumes,
    signal_handler::{add_cid, remove_cid, ContainerId, ContainerRuntime},
};

use super::{
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
    BuildDriver, DriverVersion, InspectDriver, RunDriver, SigningDriver, VerifyType,
};

const CREDS_VOL: &str = "cosign-creds";
const CREDS_PATH: &str = "/.docker/";

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
            "-t",
            opts.image.as_ref(),
            "-f",
            opts.containerfile.as_ref(),
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
        let status = cmd!(
            "docker",
            "tag",
            opts.src_image.as_ref(),
            opts.dest_image.as_ref(),
        )
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
        let status = cmd!("docker", "push", opts.image.as_ref())
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
            trace!("docker login -u {username} -p [MASKED] {registry}");
            let output = cmd!("docker", "login", "-u", username, "-p", password, registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker: {err_out}");
            }
            debug!("Logged into {registry}");
        }

        Ok(())
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<()> {
        trace!("DockerDriver::build_tag_push({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        let mut command = cmd!("docker", "buildx");

        if !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty()) {
            Self::setup()?;
            cmd!(command, "--builder=bluebuild");
        }

        cmd!(
            command,
            "build",
            "--pull",
            "-f",
            opts.containerfile.as_ref(),
        );

        // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
        if env::var(BB_BUILDKIT_CACHE_GHA).map_or_else(|_| false, |e| e == "true") {
            cmd!(
                command,
                "--cache-from",
                "type=gha",
                "--cache-to",
                "type=gha",
            );
        }

        let mut final_image = String::new();

        match (opts.image.as_ref(), opts.archive_path.as_ref()) {
            (Some(image), None) => {
                if opts.tags.is_empty() {
                    final_image.push_str(image);
                    cmd!(command, "-t", image.as_ref());
                } else {
                    final_image
                        .push_str(format!("{image}:{}", opts.tags.first().unwrap_or(&"")).as_str());

                    opts.tags.iter().for_each(|tag| {
                        cmd!(command, "-t", format!("{image}:{tag}"));
                    });
                }

                if opts.push {
                    cmd!(
                        command,
                        "--output",
                        format!(
                            "type=image,name={image},push=true,compression={},oci-mediatypes=true",
                            opts.compression
                        )
                    );
                } else {
                    cmd!(command, "--load");
                }
            }
            (None, Some(archive_path)) => {
                final_image.push_str(archive_path);

                cmd!(command, "--output", format!("type=oci,dest={archive_path}"));
            }
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        }

        cmd!(command, ".");

        trace!("{command:?}");
        if command
            .status_image_ref_progress(&final_image, "Building Image")
            .into_diagnostic()?
            .success()
        {
            if opts.push {
                info!("Successfully built and pushed image {}", final_image);
            } else {
                info!("Successfully built image {}", final_image);
            }
        } else {
            bail!("Failed to build image {}", final_image);
        }
        Ok(())
    }
}

impl InspectDriver for DockerDriver {
    fn get_metadata(opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("DockerDriver::get_labels({opts:#?})");

        let url = opts.tag.as_ref().map_or_else(
            || format!("docker://{}", opts.image),
            |tag| format!("docker://{}:{tag}", opts.image),
        );

        let progress = Logger::multi_progress().add(
            ProgressBar::new_spinner()
                .with_style(ProgressStyle::default_spinner())
                .with_message(format!("Inspecting metadata for {url}")),
        );
        progress.enable_steady_tick(Duration::from_millis(100));

        let output = Self::run_output(
            &RunOpts::builder()
                .image(SKOPEO_IMAGE)
                .args(["inspect".to_string(), url.clone()])
                .remove(true)
                .build(),
        )
        .into_diagnostic()?;

        progress.finish();
        Logger::multi_progress().remove(&progress);

        if output.status.success() {
            info!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }

        serde_json::from_slice(&output.stdout).into_diagnostic()
    }
}

impl RunDriver for DockerDriver {
    fn run(opts: &RunOpts) -> std::io::Result<ExitStatus> {
        let cid_path = TempDir::new("docker")?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerId::new(&cid_file, ContainerRuntime::Docker, false);

        add_cid(&cid);

        let status = docker_run(opts, &cid_file)
            .status_image_ref_progress(opts.image.as_ref(), "Running container")?;

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
    let mut command = cmd!("docker", "run", format!("--cidfile={}", cid_file.display()));

    if opts.privileged {
        cmd!(command, "--privileged");
    }

    if opts.remove {
        cmd!(command, "--rm");
    }

    if opts.pull {
        cmd!(command, "--pull=always");
    }

    opts.volumes.iter().for_each(|volume| {
        cmd!(
            command,
            "--volume",
            format!("{}:{}", volume.path_or_vol_name, volume.container_path,)
        );
    });

    opts.env_vars.iter().for_each(|env| {
        cmd!(command, "--env", format!("{}={}", env.key, env.value));
    });

    match (opts.uid, opts.gid) {
        (Some(uid), None) => cmd!(command, "-u", format!("{uid}")),
        (Some(uid), Some(gid)) => cmd!(command, "-u", format!("{}:{}", uid, gid)),
        _ => {}
    }

    cmd!(command, opts.image.as_ref());

    opts.args.iter().for_each(|arg| cmd!(command, arg));

    // trace!("{command:?}");
    command
}
