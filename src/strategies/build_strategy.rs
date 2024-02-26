use std::{env, path::PathBuf, process::Command};

use anyhow::{anyhow, bail, Result};
use blue_build_utils::constants::*;
use log::{info, trace};
use uuid::Uuid;

#[cfg(feature = "builtin-podman")]
use anyhow::Context;
#[cfg(feature = "futures-util")]
use futures_util::StreamExt;
#[cfg(feature = "builtin-podman")]
use log::{debug, error};
#[cfg(feature = "podman-api")]
use podman_api::{
    opts::{
        ContainerListOpts, ContainerPruneFilter, ContainerPruneOpts, ImageBuildOpts,
        ImagePruneFilter, ImagePruneOpts, ImagePushOpts, ImageTagOpts, RegistryAuth,
    },
    Podman,
};
#[cfg(feature = "signal-hook")]
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
#[cfg(feature = "signal-hook-tokio")]
use signal_hook_tokio::Signals;
#[cfg(feature = "builtin-podman")]
use std::sync::Arc;
#[cfg(feature = "tokio")]
use tokio::{
    runtime::Runtime,
    sync::oneshot::{self, Sender},
    time::{self, Duration},
};

use crate::commands::build::Credentials;

#[derive(Debug)]
enum Strategy {
    Buildah,
    Podman,
    #[cfg(feature = "builtin-podman")]
    Socket(Arc<Podman>, Runtime),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BuildStrategy<'a> {
    strategy: Strategy,
    uuid: Uuid,
    creds: Option<&'a Credentials>,
}

impl<'a> BuildStrategy<'a> {
    pub fn determine_strategy(uuid: Uuid, creds: Option<&'a Credentials>) -> Result<Self> {
        trace!("BuildStrategy::determine_strategy({uuid})");

        Ok(Self {
            strategy: match (
                env::var(XDG_RUNTIME_DIR),
                PathBuf::from(RUN_PODMAN_SOCK),
                PathBuf::from(VAR_RUN_PODMAN_PODMAN_SOCK),
                PathBuf::from(VAR_RUN_PODMAN_SOCK),
                blue_build_utils::check_command_exists("podman"),
                blue_build_utils::check_command_exists("buildah"),
            ) {
                #[cfg(feature = "builtin-podman")]
                (Ok(xdg_runtime), _, _, _, _, _)
                    if PathBuf::from(format!("{xdg_runtime}/podman/podman.sock")).exists() =>
                {
                    Strategy::Socket(
                        Podman::unix(PathBuf::from(format!("{xdg_runtime}/podman/podman.sock")))
                            .into(),
                        Runtime::new()?,
                    )
                }
                #[cfg(feature = "builtin-podman")]
                (_, run_podman_podman_sock, _, _, _, _) if run_podman_podman_sock.exists() => {
                    Strategy::Socket(Podman::unix(run_podman_podman_sock).into(), Runtime::new()?)
                }
                #[cfg(feature = "builtin-podman")]
                (_, _, var_run_podman_podman_sock, _, _, _)
                    if var_run_podman_podman_sock.exists() =>
                {
                    Strategy::Socket(
                        Podman::unix(var_run_podman_podman_sock).into(),
                        Runtime::new()?,
                    )
                }
                #[cfg(feature = "builtin-podman")]
                (_, _, _, var_run_podman_sock, _, _) if var_run_podman_sock.exists() => {
                    Strategy::Socket(Podman::unix(var_run_podman_sock).into(), Runtime::new()?)
                }
                (_, _, _, _, Ok(()), _) => Strategy::Podman,
                (_, _, _, _, _, Ok(())) => Strategy::Buildah,
                _ => bail!("Could not determine strategy"),
            },
            uuid,
            creds,
        })
    }

    pub fn build(&self, image: &str) -> Result<()> {
        match &self.strategy {
            Strategy::Podman => {
                trace!("podman build . -t {image}");
                let status = Command::new("podman")
                    .arg("build")
                    .arg(".")
                    .arg("-t")
                    .arg(image)
                    .status()?;

                if status.success() {
                    info!("Successfully built {image}");
                } else {
                    bail!("Failed to build {image}");
                }
            }
            Strategy::Buildah => {
                trace!("buildah build -t {image}");
                let status = Command::new("buildah")
                    .arg("build")
                    .arg("-t")
                    .arg(image)
                    .status()?;

                if status.success() {
                    info!("Successfully built {image}");
                } else {
                    bail!("Failed to build {image}");
                }
            }
            #[cfg(feature = "builtin-podman")]
            Strategy::Socket(client, rt) => {
                rt.block_on(async {
                    let signals = Signals::new([SIGTERM, SIGINT, SIGQUIT])?;
                    let handle = signals.handle();

                    let (kill_tx, mut kill_rx) = oneshot::channel::<()>();

                    let signals_task =
                        tokio::spawn(handle_signals(signals, kill_tx, self.uuid, client.clone()));

                    // Get podman ready to build
                    let opts = ImageBuildOpts::builder(".")
                        .tag(image)
                        .dockerfile("Containerfile")
                        .remove(true)
                        .layers(true)
                        .labels([(BUILD_ID_LABEL, self.uuid.to_string())])
                        .pull(true)
                        .build();
                    trace!("Build options: {opts:#?}");

                    info!("Building image {image}");
                    match client.images().build(&opts) {
                        Ok(mut build_stream) => loop {
                            tokio::select! {
                                Some(chunk) = build_stream.next() => {
                                    match chunk {
                                        Ok(chunk) => chunk
                                            .stream
                                            .trim()
                                            .lines()
                                            .map(str::trim)
                                            .filter(|line| !line.is_empty())
                                            .for_each(|line| info!("{line}")),
                                        Err(e) => bail!("{e}"),
                                    }
                                },
                                _ = &mut kill_rx => {
                                    break;
                                },
                                else => {
                                    break;
                                }
                            }
                        },
                        Err(e) => bail!("{e}"),
                    };
                    handle.close();
                    signals_task.await?;
                    Ok(())
                })?;
            }
        };
        Ok(())
    }

    pub fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()> {
        let dest_image = format!("{image_name}:{tag}");

        match &self.strategy {
            Strategy::Podman => {
                trace!("podman tag {src_image} {dest_image}");
                let status = Command::new("podman")
                    .arg("tag")
                    .arg(src_image)
                    .arg(&dest_image)
                    .status()?;

                if status.success() {
                    info!("Successfully tagged {dest_image}!");
                } else {
                    bail!("Failed to tag image {dest_image}");
                }
            }
            Strategy::Buildah => {
                trace!("buildah tag {src_image} {dest_image}");
                let status = Command::new("buildah")
                    .arg("tag")
                    .arg(src_image)
                    .arg(&dest_image)
                    .status()?;

                if status.success() {
                    info!("Successfully tagged {dest_image}!");
                } else {
                    bail!("Failed to tag image {dest_image}");
                }
            }
            #[cfg(feature = "builtin-podman")]
            Strategy::Socket(client, rt) => {
                let first_image = client.images().get(src_image);
                rt.block_on(async {
                    first_image
                        .tag(&ImageTagOpts::builder().repo(image_name).tag(tag).build())
                        .await
                        .context("Failed to tag image")?;
                    debug!("Tagged image {dest_image}");
                    Ok::<(), anyhow::Error>(())
                })?;
            }
        };
        Ok(())
    }

    pub fn push(&self, image: &str) -> Result<()> {
        match &self.strategy {
            Strategy::Podman => {
                trace!("podman push {image}");
                let status = Command::new("podman").arg("push").arg(image).status()?;

                if status.success() {
                    info!("Successfully pushed {image}!");
                } else {
                    bail!("Failed to push image {image}")
                }
            }
            Strategy::Buildah => {
                trace!("buildah push {image}");
                let status = Command::new("buildah").arg("push").arg(image).status()?;

                if status.success() {
                    info!("Successfully pushed {image}!");
                } else {
                    bail!("Failed to push image {image}")
                }
            }
            #[cfg(feature = "builtin-podman")]
            Strategy::Socket(client, rt) => {
                let (username, password, registry) = self
                    .creds
                    .map(|c| (&c.username, &c.password, &c.registry))
                    .ok_or_else(|| anyhow!("No credentials provided, unable to push"))?;

                rt.block_on(async {
                    let new_image = client.images().get(image);
                    info!("Pushing {image}");
                    match new_image
                        .push(
                            &ImagePushOpts::builder()
                                .tls_verify(true)
                                .auth(
                                    RegistryAuth::builder()
                                        .username(username)
                                        .password(password)
                                        .server_address(registry)
                                        .build(),
                                )
                                .build(),
                        )
                        .await
                    {
                        Ok(_) => info!("Pushed {image} successfully!"),
                        Err(e) => bail!("Failed to push image: {e}"),
                    };
                    Ok(())
                })?;
            }
        };
        Ok(())
    }

    pub fn login(&self) -> Result<()> {
        let (registry, username, password) = self
            .creds
            .map(|credentials| {
                (
                    &credentials.registry,
                    &credentials.username,
                    &credentials.password,
                )
            })
            .ok_or_else(|| anyhow!("Unable to login, missing credentials!"))?;

        match &self.strategy {
            Strategy::Podman => {
                trace!("podman login -u {username} -p [MASKED] {registry}");
                let output = Command::new("podman")
                    .arg("login")
                    .arg("-u")
                    .arg(username)
                    .arg("-p")
                    .arg(password)
                    .arg(registry)
                    .output()?;

                if !output.status.success() {
                    let err_out = String::from_utf8_lossy(&output.stderr);
                    bail!("Failed to login for buildah: {err_out}");
                }
            }
            Strategy::Buildah => {
                trace!("buildah login -u {username} -p [MASKED] {registry}");
                let output = Command::new("buildah")
                    .arg("login")
                    .arg("-u")
                    .arg(username)
                    .arg("-p")
                    .arg(password)
                    .arg(registry)
                    .output()?;

                if !output.status.success() {
                    let err_out = String::from_utf8_lossy(&output.stderr);
                    bail!("Failed to login for buildah: {err_out}");
                }
            }
            #[cfg(feature = "builtin-podman")]
            Strategy::Socket(_, _) => {
                debug!("No login step for Socket based building, skipping...");
            }
        };
        Ok(())
    }
}

#[cfg(feature = "builtin-podman")]
async fn handle_signals(
    mut signals: Signals,
    kill: Sender<()>,
    build_id: Uuid,
    client: Arc<Podman>,
) {
    use std::process;

    trace!("handle_signals(signals, {build_id}, {client:#?})");

    while let Some(signal) = signals.next().await {
        match signal {
            SIGHUP => (),
            SIGINT => {
                kill.send(()).unwrap();
                info!("Recieved SIGINT, cleaning up build...");

                time::sleep(Duration::from_secs(1)).await;

                let containers = match client
                    .containers()
                    .list(&ContainerListOpts::builder().sync(true).all(true).build())
                    .await
                {
                    Ok(list) => list,
                    Err(e) => {
                        error!("{e}");
                        process::exit(1);
                    }
                };

                trace!("{containers:#?}");

                // Prune containers from this build
                let container_prune_opts = ContainerPruneOpts::builder()
                    .filter([ContainerPruneFilter::LabelKeyVal(
                        BUILD_ID_LABEL.to_string(),
                        build_id.to_string(),
                    )])
                    .build();
                if let Err(e) = client.containers().prune(&container_prune_opts).await {
                    error!("{e}");
                    process::exit(1);
                }
                debug!("Pruned containers");

                // Prune images from this build
                let image_prune_opts = ImagePruneOpts::builder()
                    .filter([ImagePruneFilter::LabelKeyVal(
                        BUILD_ID_LABEL.to_string(),
                        build_id.to_string(),
                    )])
                    .build();
                if let Err(e) = client.images().prune(&image_prune_opts).await {
                    error!("{e}");
                    process::exit(1);
                }
                debug!("Pruned images");
                process::exit(2);
            }
            _ => unreachable!(),
        }
    }
}
