use std::{
    env,
    path::Path,
    process::{Command, ExitStatus},
    sync::Mutex,
    time::Duration,
};

use blue_build_utils::{
    constants::{BB_BUILDKIT_CACHE_GHA, CONTAINER_FILE, DOCKER_HOST, SKOPEO_IMAGE},
    logging::{CommandLogging, Logger},
    signal_handler::{add_cid, remove_cid, ContainerId},
};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, trace, warn};
use miette::{bail, IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use semver::Version;
use serde::Deserialize;
use tempdir::TempDir;

use crate::{
    credentials::Credentials, drivers::types::RunDriverType, image_metadata::ImageMetadata,
};

use super::{
    credentials,
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, RunOpts, TagOpts},
    BuildDriver, DriverVersion, InspectDriver, RunDriver,
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
        let ls_out = Command::new("docker")
            .arg("buildx")
            .arg("ls")
            .arg("--format={{.Name}}")
            .output()
            .into_diagnostic()?;

        if !ls_out.status.success() {
            bail!("{}", String::from_utf8_lossy(&ls_out.stderr));
        }

        let ls_out = String::from_utf8(ls_out.stdout).into_diagnostic()?;

        trace!("{ls_out}");

        if !ls_out.lines().any(|line| line == "bluebuild") {
            trace!("docker buildx create --bootstrap --driver=docker-container --name=bluebuild");
            let create_out = Command::new("docker")
                .arg("buildx")
                .arg("create")
                .arg("--bootstrap")
                .arg("--driver=docker-container")
                .arg("--name=bluebuild")
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
        let output = Command::new("docker")
            .arg("version")
            .arg("-f")
            .arg("json")
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
        let status = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(opts.image.as_ref())
            .arg("-f")
            .arg(opts.containerfile.as_ref())
            .arg(".")
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
        let status = Command::new("docker")
            .arg("tag")
            .arg(opts.src_image.as_ref())
            .arg(opts.dest_image.as_ref())
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
        let status = Command::new("docker")
            .arg("push")
            .arg(opts.image.as_ref())
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
        }) = credentials::get()
        {
            trace!("docker login -u {username} -p [MASKED] {registry}");
            let output = Command::new("docker")
                .arg("login")
                .arg("-u")
                .arg(username)
                .arg("-p")
                .arg(password)
                .arg(registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker: {err_out}");
            }
        }
        Ok(())
    }

    fn build_tag_push(opts: &BuildTagPushOpts) -> Result<()> {
        trace!("DockerDriver::build_tag_push({opts:#?})");

        if opts.squash {
            warn!("Squash is deprecated for docker so this build will not squash");
        }

        trace!("docker buildx");
        let mut command = Command::new("docker");
        command.arg("buildx");

        if !env::var(DOCKER_HOST).is_ok_and(|dh| !dh.is_empty()) {
            Self::setup()?;

            trace!("--builder=bluebuild");
            command.arg("--builder=bluebuild");
        }

        trace!(
            "build --progress=plain --pull -f {}",
            opts.containerfile.display()
        );
        command
            .arg("build")
            .arg("--pull")
            .arg("-f")
            .arg(opts.containerfile.as_ref());

        // https://github.com/moby/buildkit?tab=readme-ov-file#github-actions-cache-experimental
        if env::var(BB_BUILDKIT_CACHE_GHA).map_or_else(|_| false, |e| e == "true") {
            trace!("--cache-from type=gha --cache-to type=gha");
            command
                .arg("--cache-from")
                .arg("type=gha")
                .arg("--cache-to")
                .arg("type=gha");
        }

        let mut final_image = String::new();

        match (opts.image.as_ref(), opts.archive_path.as_ref()) {
            (Some(image), None) => {
                if opts.tags.is_empty() {
                    final_image.push_str(image);

                    trace!("-t {image}");
                    command.arg("-t").arg(image.as_ref());
                } else {
                    final_image
                        .push_str(format!("{image}:{}", opts.tags.first().unwrap_or(&"")).as_str());

                    opts.tags.iter().for_each(|tag| {
                        let full_image = format!("{image}:{tag}");

                        trace!("-t {full_image}");
                        command.arg("-t").arg(full_image);
                    });
                }

                if opts.push {
                    trace!("--output type=image,name={image},push=true,compression={},oci-mediatypes=true", opts.compression);
                    command.arg("--output").arg(format!(
                        "type=image,name={image},push=true,compression={},oci-mediatypes=true",
                        opts.compression
                    ));
                } else {
                    trace!("--load");
                    command.arg("--load");
                }
            }
            (None, Some(archive_path)) => {
                final_image.push_str(archive_path);

                trace!("--output type=oci,dest={archive_path}");
                command
                    .arg("--output")
                    .arg(format!("type=oci,dest={archive_path}"));
            }
            (Some(_), Some(_)) => bail!("Cannot use both image and archive path"),
            (None, None) => bail!("Need either the image or archive path set"),
        }

        trace!(".");
        command.arg(".");

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
                .args(&["inspect".to_string(), url.clone()])
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
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new("docker")?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerId::new(&cid_file, RunDriverType::Docker, false);

        add_cid(&cid);

        let status = docker_run(opts, &cid_file)
            .status_image_ref_progress(opts.image.as_ref(), "Running container")?;

        remove_cid(&cid);

        Ok(status)
    }

    fn run_output(opts: &RunOpts) -> std::io::Result<std::process::Output> {
        trace!("DockerDriver::run({opts:#?})");

        let cid_path = TempDir::new("docker")?;
        let cid_file = cid_path.path().join("cid");
        let cid = ContainerId::new(&cid_file, RunDriverType::Docker, false);

        add_cid(&cid);

        let output = docker_run(opts, &cid_file).output()?;

        remove_cid(&cid);

        Ok(output)
    }
}

fn docker_run(opts: &RunOpts, cid_file: &Path) -> Command {
    let mut command = Command::new("docker");

    command
        .arg("run")
        .arg(format!("--cidfile={}", cid_file.display()));

    if opts.privileged {
        command.arg("--privileged");
    }

    if opts.remove {
        command.arg("--rm");
    }

    if opts.pull {
        command.arg("--pull=always");
    }

    opts.volumes.iter().for_each(|volume| {
        command.arg("--volume");
        command.arg(format!(
            "{}:{}",
            volume.path_or_vol_name, volume.container_path,
        ));
    });

    opts.env_vars.iter().for_each(|env| {
        command.arg("--env");
        command.arg(format!("{}={}", env.key, env.value));
    });

    command.arg(opts.image.as_ref());

    command.args(opts.args.iter());

    trace!("{command:?}");
    command
}
