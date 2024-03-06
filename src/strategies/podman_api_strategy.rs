use anyhow::Context;
use anyhow::{anyhow, bail, Result};
use blue_build_utils::constants::*;
use futures_util::StreamExt;
use log::{debug, error};
use log::{info, trace};
use podman_api::{
    opts::{
        ContainerListOpts, ContainerPruneFilter, ContainerPruneOpts, ImageBuildOpts,
        ImagePruneFilter, ImagePruneOpts, ImagePushOpts, ImageTagOpts, RegistryAuth,
    },
    Podman,
};
use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook_tokio::Signals;
use std::sync::Arc;
use tokio::{
    runtime::Runtime,
    sync::oneshot::{self, Sender},
    time::{self, Duration},
};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use super::{BuildStrategy, Credentials};

#[derive(Debug, TypedBuilder)]
pub struct PodmanApiStrategy {
    client: Arc<Podman>,
    rt: Runtime,
    uuid: Uuid,
    creds: Option<Credentials>,
}

impl BuildStrategy for PodmanApiStrategy {
    fn build(&self, image: &str) -> Result<()> {
        self.rt.block_on(async {
            let signals = Signals::new([SIGTERM, SIGINT, SIGQUIT])?;
            let handle = signals.handle();

            let (kill_tx, mut kill_rx) = oneshot::channel::<()>();

            let signals_task = tokio::spawn(handle_signals(
                signals,
                kill_tx,
                self.uuid,
                self.client.clone(),
            ));

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
            match self.client.images().build(&opts) {
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
        })
    }

    fn tag(&self, src_image: &str, image_name: &str, tag: &str) -> Result<()> {
        let first_image = self.client.images().get(src_image);
        self.rt.block_on(async {
            first_image
                .tag(&ImageTagOpts::builder().repo(image_name).tag(tag).build())
                .await
                .context("Failed to tag image")?;
            debug!("Tagged image {image_name}:{tag}");
            Ok(())
        })
    }

    fn push(&self, image: &str) -> Result<()> {
        let (username, password, registry) = self
            .creds
            .as_ref()
            .map(|c| (&c.username, &c.password, &c.registry))
            .ok_or_else(|| anyhow!("No credentials provided, unable to push"))?;

        self.rt.block_on(async {
            let new_image = self.client.images().get(image);
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
        })
    }

    fn login(&self) -> Result<()> {
        debug!("No login step for Socket based building, skipping...");
        Ok(())
    }

    fn inspect(&self, image_name: &str, tag: &str) -> Result<Vec<u8>> {
        todo!()
    }
}

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
