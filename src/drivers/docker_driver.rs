use std::{
    env,
    process::{Command, Stdio},
    sync::Mutex,
};

use anyhow::{anyhow, bail, Result};
use blue_build_utils::{
    constants::{BB_BUILDKIT_CACHE_GHA, CONTAINER_FILE, DOCKER_HOST, SKOPEO_IMAGE},
    logging::{shorten_image_names, CommandLogging},
};
use log::{info, trace, warn};
use once_cell::sync::Lazy;
use semver::Version;
use serde::Deserialize;

use crate::image_metadata::ImageMetadata;

use super::{
    credentials,
    opts::{BuildOpts, BuildTagPushOpts, GetMetadataOpts, PushOpts, TagOpts},
    BuildDriver, DriverVersion, InspectDriver,
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

        let mut lock = DOCKER_SETUP
            .lock()
            .map_err(|e| anyhow!("Failed to lock DOCKER_SETUP: {e}"))?;

        if *lock {
            drop(lock);
            return Ok(());
        }

        trace!("docker buildx ls --format={}", "{{.Name}}");
        let ls_out = Command::new("docker")
            .arg("buildx")
            .arg("ls")
            .arg("--format={{.Name}}")
            .output()?;

        if !ls_out.status.success() {
            bail!("{}", String::from_utf8_lossy(&ls_out.stderr));
        }

        let ls_out = String::from_utf8(ls_out.stdout)?;

        trace!("{ls_out}");

        if !ls_out.lines().any(|line| line == "bluebuild") {
            trace!("docker buildx create --bootstrap --driver=docker-container --name=bluebuild");
            let create_out = Command::new("docker")
                .arg("buildx")
                .arg("create")
                .arg("--bootstrap")
                .arg("--driver=docker-container")
                .arg("--name=bluebuild")
                .output()?;

            if create_out.status.success() {
                *lock = true;
            } else {
                bail!("{}", String::from_utf8_lossy(&create_out.stderr));
            }
        }
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
            .output()?;

        let version_json: DockerVersionJson = serde_json::from_slice(&output.stdout)?;

        Ok(version_json.client.version)
    }
}

impl BuildDriver for DockerDriver {
    fn build(&self, opts: &BuildOpts) -> Result<()> {
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
            .status()?;

        if status.success() {
            info!("Successfully built {}", opts.image);
        } else {
            bail!("Failed to build {}", opts.image);
        }
        Ok(())
    }

    fn tag(&self, opts: &TagOpts) -> Result<()> {
        trace!("DockerDriver::tag({opts:#?})");

        trace!("docker tag {} {}", opts.src_image, opts.dest_image);
        let status = Command::new("docker")
            .arg("tag")
            .arg(opts.src_image.as_ref())
            .arg(opts.dest_image.as_ref())
            .status()?;

        if status.success() {
            info!("Successfully tagged {}!", opts.dest_image);
        } else {
            bail!("Failed to tag image {}", opts.dest_image);
        }
        Ok(())
    }

    fn push(&self, opts: &PushOpts) -> Result<()> {
        trace!("DockerDriver::push({opts:#?})");

        trace!("docker push {}", opts.image);
        let status = Command::new("docker")
            .arg("push")
            .arg(opts.image.as_ref())
            .status()?;

        if status.success() {
            info!("Successfully pushed {}!", opts.image);
        } else {
            bail!("Failed to push image {}", opts.image);
        }
        Ok(())
    }

    fn login(&self) -> Result<()> {
        trace!("DockerDriver::login()");

        let (registry, username, password) =
            credentials::get().map(|c| (&c.registry, &c.username, &c.password))?;

        trace!("docker login -u {username} -p [MASKED] {registry}");
        let output = Command::new("docker")
            .arg("login")
            .arg("-u")
            .arg(username)
            .arg("-p")
            .arg(password)
            .arg(registry)
            .output()?;

        if !output.status.success() {
            let err_out = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to login for docker: {err_out}");
        }
        Ok(())
    }

    fn build_tag_push(&self, opts: &BuildTagPushOpts) -> Result<()> {
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
            .arg("--progress=plain")
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
            .status_log_prefix(&shorten_image_names(&final_image))?
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
    fn get_metadata(&self, opts: &GetMetadataOpts) -> Result<ImageMetadata> {
        trace!("DockerDriver::get_labels({opts:#?})");

        let url = opts.tag.as_ref().map_or_else(
            || format!("docker://{}", opts.image),
            |tag| format!("docker://{}:{tag}", opts.image),
        );

        trace!("docker run {SKOPEO_IMAGE} inspect {url}");
        let output = Command::new("docker")
            .arg("run")
            .arg("--rm")
            .arg(SKOPEO_IMAGE)
            .arg("inspect")
            .arg(&url)
            .stderr(Stdio::inherit())
            .output()?;

        if output.status.success() {
            info!("Successfully inspected image {url}!");
        } else {
            bail!("Failed to inspect image {url}")
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }
}
