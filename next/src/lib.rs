//! This library is the Next Generation Build Engine for BlueBuild.
//!
//! This build engine is based on `buildah` and allows building a
//! directed acyclical graph (DAG) from a BlueBuild recipe. This
//! engine takes advantage of OCI per-layer operations
//! that allow us  evaluate the state of the image mid-build that a
//! standard `Containerfile` is not capable of.
//!
#![doc = include_str!("../LGPL_2.1.txt")]

mod build_scripts;
pub mod layer;
pub mod path;
pub mod reference;

use std::{collections::HashMap, iter::once, path::Path, process::Command, sync::Arc};

use blue_build_recipe::{ModuleRequiredFields, Recipe, RecipeGetters, StageRequiredFields};
use blue_build_utils::{constants::NUSHELL_IMAGE, platform::Platform};
use futures::{
    future::{self},
    stream::{self, TryStreamExt},
};
use miette::{IntoDiagnostic, Result, bail};

use crate::{
    layer::{CopyLayer, FinalizeLayer, FromLayer, Layer, LayerId, Mount, MountMode, RunLayer},
    reference::PinnedReference,
};

#[derive(Debug, Clone)]
pub struct Manifest {
    images: HashMap<Platform, Image>,
}

#[bon::bon]
impl Manifest {
    #[builder]
    pub async fn new(
        recipe: &Recipe,
        build_scripts_layer: &LayerId,
        squash: bool,
        context_dir: &Path,
    ) -> Result<Self> {
        let platforms = recipe.get_platforms();
        let create_image = async |platform| -> Result<(Platform, Image)> {
            let image_plan = Image::builder()
                .recipe(recipe)
                .platform(platform)
                .squash(squash)
                .context_dir(context_dir)
                .build_scripts_layer(build_scripts_layer)
                .build()
                .await?;
            Ok((platform, image_plan))
        };

        let images = future::try_join_all(platforms.iter().copied().map(create_image))
            .await?
            .into_iter()
            .collect();

        Ok(Self { images })
    }

    /// Runs a build for all platforms
    /// in an image.
    ///
    /// # Errors
    /// Will error if any of the builds fail.
    pub async fn build(self) -> Result<HashMap<Platform, LayerId>> {
        future::try_join_all(self.images.into_iter().map(|(platform, image)| {
            tokio::spawn(async move { image.build().await.map(|layer| (platform, layer)) })
        }))
        .await
        .into_diagnostic()?
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    layers: Layer,
}

#[bon::bon]
impl Image {
    #[builder]
    pub async fn new(
        /// The recipe image to build.
        recipe: &Recipe,

        /// The platform to build.
        #[builder(default)]
        platform: Platform,

        /// Squash the build into a single
        /// layer on the base image.
        #[builder(default)]
        squash: bool,

        /// The layer containing the build scripts.
        build_scripts_layer: &LayerId,

        /// The build's context directory.
        context_dir: &Path,
    ) -> Result<Self> {
        let image = PinnedReference::pin_image(recipe.base_image_ref()?).await?;
        let from = FromLayer::builder()
            .from(image)
            .platform(platform)
            .squash(squash)
            .build();
        let keys = RunLayer::builder()
            .parent(from)
            .mounts([keys_mnt(recipe, context_dir)?])
            .command("mkdir -p /etc/pki/containers/ && cp /tmp/keys/* /etc/pki/containers/")
            .build();
        let stages = recipe
            .get_processed_stages()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let layers = stream::iter(
            recipe
                .get_processed_modules()
                .into_iter()
                .map(Ok::<_, miette::Error>),
        )
        .try_fold(keys.into(), move |parent, module| {
            let stages = stages.clone();
            async move {
                add_module(
                    parent,
                    module,
                    recipe,
                    build_scripts_layer,
                    context_dir,
                    &stages,
                    &[],
                )
                .await
            }
        })
        .await?;
        Ok(Self { layers })
    }

    /// Build the image.
    ///
    /// # Errors
    /// Will error if the build fails.
    pub async fn build(self) -> Result<LayerId> {
        self.layers.finalize().await
    }
}

async fn add_module<'a>(
    parent: Layer,
    module: &ModuleRequiredFields,
    recipe: &'a Recipe,
    build_scripts_layer: &'a LayerId,
    context_dir: &'a Path,
    stages: &[StageRequiredFields],
    traversed_stages: &[String],
) -> Result<Layer> {
    Ok(match module.module_type.typ() {
        "copy" => {
            let Some(source) = module.config["src"].as_str() else {
                bail!("The `src` property was expected on the copy module.");
            };
            let Some(destination) = module.config["dest"].as_str() else {
                bail!("The `dest` property was expected on the copy module.");
            };
            let from = if let Some(stage) = module.config["from"]
                .as_str()
                .and_then(|from| stages.iter().find(|stage| stage.name == from))
            {
                Some(
                    create_stage(
                        recipe,
                        build_scripts_layer,
                        context_dir,
                        stages,
                        traversed_stages,
                        stage,
                    )
                    .await?,
                )
            } else {
                None
            };
            CopyLayer::builder()
                .parent(parent)
                .maybe_from(from)
                .source(context_dir, source)?
                .destination(destination)
                .build()
                .into()
        }
        "containerfile" => panic!("The 'containerfile' type is not supported"),
        _ => {
            let mounts = [
                build_scripts_mnt(build_scripts_layer),
                files_mnt(context_dir)?,
                nushell_mnt().await?,
                modules_mnt(context_dir, module).await?,
            ];
            RunLayer::builder()
                .parent(parent)
                .args([
                    ("CONFIG_DIRECTORY".to_string(), "/tmp/files".to_string()),
                    ("MODULE_DIRECTORY".to_string(), "/tmp/modules".to_string()),
                    ("IMAGE_NAME".to_string(), recipe.get_name().to_string()),
                    (
                        "BASE_IMAGE".to_string(),
                        recipe.get_base_image().to_string(),
                    ),
                    (
                        "IMAGE_REGISTRY".to_string(),
                        // TODO: replace with registry
                        "ghcr.io/blue-build/cli".to_string(),
                    ),
                    ("BB_BUILD_FEATURES".to_string(), String::new()),
                ])
                .command(Command::try_from(module)?)
                .mounts(mounts)
                .build()
                .into()
        }
    })
}

fn build_scripts_mnt(build_scripts_layer: &LayerId) -> Arc<Mount> {
    Mount::new_bind()
        .source((build_scripts_layer.clone(), "/scripts"))
        .destination("/tmp/scripts")
        .build()
}

async fn nushell_mnt() -> Result<Arc<Mount>> {
    let image = PinnedReference::pin_image(
        format!("{NUSHELL_IMAGE}:default")
            .parse()
            .into_diagnostic()?,
    )
    .await?;
    Ok(Mount::new_bind()
        .source((FromLayer::builder().from(image).build(), "/nu"))
        .destination("/usr/libexec/bluebuild/nu")
        .build())
}

fn files_mnt(context_dir: &Path) -> Result<Arc<Mount>> {
    Ok(Mount::new_bind()
        .source((
            CopyLayer::builder()
                .parent(FromLayer::builder().build())
                .source(context_dir, "./files")?
                .destination("/files")
                .build(),
            "/files",
        ))
        .destination("/tmp/files")
        .build())
}

fn keys_mnt(recipe: &Recipe, context_dir: &Path) -> Result<Arc<Mount>> {
    Ok(Mount::new_bind()
        .source((
            CopyLayer::builder()
                .parent(FromLayer::builder().build())
                .source(context_dir, "cosign.pub")?
                .destination(format!("/keys/{}.pub", recipe.get_name().replace('/', "_")))
                .build(),
            "/keys",
        ))
        .destination("/tmp/keys")
        .build())
}

async fn modules_mnt(context_dir: &Path, module: &ModuleRequiredFields) -> Result<Arc<Mount>> {
    Ok(if let Some(source) = module.get_non_local_source() {
        let source = PinnedReference::pin_image(source.parse().into_diagnostic()?).await?;
        Mount::new_bind()
            .source((FromLayer::builder().from(source).build(), "/modules"))
            .destination("/tmp/modules")
            .mode(MountMode::ReadWrite)
            .build()
    } else if module.is_local_source() {
        Mount::new_bind()
            .source((
                CopyLayer::builder()
                    .parent(FromLayer::builder().build())
                    .source(context_dir, "./modules")?
                    .destination("/modules")
                    .build(),
                "/modules",
            ))
            .destination("/tmp/modules")
            .mode(MountMode::Default)
            .build()
    } else {
        let image =
            PinnedReference::pin_image(module.get_module_image().parse().into_diagnostic()?)
                .await?;
        Mount::new_bind()
            .source((FromLayer::builder().from(image).build(), "/modules"))
            .destination("/tmp/modules")
            .mode(MountMode::ReadWrite)
            .build()
    })
}

async fn create_stage(
    recipe: &Recipe,
    build_scripts_layer: &LayerId,
    context_dir: &Path,
    stages: &[StageRequiredFields],
    traversed_stages: &[String],
    stage: &StageRequiredFields,
) -> Result<Layer> {
    if traversed_stages.contains(&stage.name) {
        bail!("Hit cycle in build graph:\n{traversed_stages:?}");
    }
    let traversed_stages = Arc::new(
        once(stage.name.clone())
            .chain(traversed_stages.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let image = PinnedReference::pin_image(stage.from.parse().into_diagnostic()?).await?;
    let from = FromLayer::builder()
        .from(image)
        .maybe_platform(stage.platform)
        .build();

    stream::iter(stage.get_processed_modules().into_iter().map(Ok))
        .try_fold(from.into(), move |parent, module| {
            let traversed_stages = traversed_stages.clone();
            async move {
                // We pin the future since this is where the recursion starts.
                Box::pin(add_module(
                    parent,
                    module,
                    recipe,
                    build_scripts_layer,
                    context_dir,
                    stages,
                    &traversed_stages,
                ))
                .await
            }
        })
        .await
}

#[cfg(test)]
mod test {
    use std::sync::LazyLock;

    use crate::build_scripts::build_scripts_layer;

    use super::*;
    use rstest::rstest;

    static CONTEXT_DIR: LazyLock<&Path> = LazyLock::new(|| Path::new("./tests/repo/"));

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[rstest]
    async fn image() {
        let build_scripts_layer = build_scripts_layer().await.unwrap();
        let recipe = Recipe::builder()
            .path("./tests/repo/recipes/recipe.yml")
            .build()
            .unwrap();
        let image = Image::builder()
            .recipe(&recipe)
            .context_dir(&CONTEXT_DIR)
            .build_scripts_layer(&build_scripts_layer)
            .build()
            .await
            .unwrap();

        dbg!(&image);

        assert!(image.build().await.is_ok());
    }
}
