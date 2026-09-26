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

use std::{collections::HashMap, iter::once, path::Path, process::Command};

use blue_build_recipe::{ModuleRequiredFields, Recipe, RecipeGetters, StageRequiredFields};
use blue_build_utils::{constants::NUSHELL_IMAGE, platform::Platform};
use miette::{Result, bail};
use rayon::prelude::*;

use crate::{
    build_scripts::build_scripts_layer,
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
    pub fn new(recipe: &Recipe, squash: bool, context_dir: &Path) -> Result<Self> {
        let platforms = recipe.get_platforms();
        let create_image = |platform| {
            let image_plan = Image::builder()
                .recipe(recipe)
                .platform(platform)
                .squash(squash)
                .context_dir(context_dir)
                .build()?;
            Ok((platform, image_plan))
        };

        Ok(Self {
            images: platforms
                .iter()
                .copied()
                .map(create_image)
                .collect::<Result<HashMap<_, _>>>()?,
        })
    }

    /// Runs a build for all platforms
    /// in an image.
    ///
    /// # Errors
    /// Will error if any of the builds fail.
    pub fn build(self) -> Result<HashMap<Platform, LayerId>> {
        self.images
            .into_par_iter()
            .map(|(platform, image)| Ok((platform, image.build()?)))
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
    pub fn new(
        /// The recipe image to build.
        recipe: &Recipe,

        /// The platform to build.
        #[builder(default)]
        platform: Platform,

        /// Squash the build into a single
        /// layer on the base image.
        #[builder(default)]
        squash: bool,

        /// The build's context directory.
        context_dir: &Path,
    ) -> Result<Self> {
        let from = FromLayer::builder()
            .from(
                recipe
                    .base_image_ref()
                    .and_then(PinnedReference::try_from)?,
            )
            .platform(platform)
            .squash(squash)
            .build();
        let keys = RunLayer::builder()
            .parent(from)
            .mounts([keys_mnt(recipe, context_dir)?])
            .command("mkdir -p /etc/pki/containers/ && cp /tmp/keys/* /etc/pki/containers/")
            .build();
        let layers = recipe
            .get_processed_modules()
            .iter()
            .try_fold(keys.into(), add_module(recipe, context_dir, Vec::new()))?;
        Ok(Self { layers })
    }

    /// Build the image.
    ///
    /// # Errors
    /// Will error if the build fails.
    pub fn build(self) -> Result<LayerId> {
        self.layers.finalize()
    }
}

fn add_module(
    recipe: &Recipe,
    context_dir: &Path,
    traversed_stages: Vec<&String>,
) -> impl FnMut(Layer, &&ModuleRequiredFields) -> Result<Layer> {
    let stages = recipe.get_processed_stages();
    move |parent, module| {
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
                    create_stage(recipe, context_dir, &traversed_stages, stage)?
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
            _ => RunLayer::builder()
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
                .command(Command::try_from(*module)?)
                .mounts([
                    build_scripts_mnt(&build_scripts_layer()?),
                    modules_mnt(context_dir, module)?,
                    files_mnt(context_dir)?,
                    nushell_mnt()?,
                ])
                .build()
                .into(),
        })
    }
}

fn build_scripts_mnt(build_scripts_layer: &LayerId) -> Mount {
    Mount::new_bind()
        .source((build_scripts_layer.clone(), "/scripts"))
        .destination("/tmp/scripts")
        .build()
}

fn nushell_mnt() -> Result<Mount> {
    Ok(Mount::new_bind()
        .source((
            FromLayer::builder()
                .from(PinnedReference::try_from(&format!(
                    "{NUSHELL_IMAGE}:default"
                ))?)
                .build(),
            "/nu",
        ))
        .destination("/usr/libexec/bluebuild/nu")
        .build())
}

fn files_mnt(context_dir: &Path) -> Result<Mount> {
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

fn keys_mnt(recipe: &Recipe, context_dir: &Path) -> Result<Mount> {
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

fn modules_mnt(context_dir: &Path, module: &ModuleRequiredFields) -> Result<Mount> {
    Ok(if let Some(source) = module.get_non_local_source() {
        Mount::new_bind()
            .source((
                FromLayer::builder()
                    .from(PinnedReference::try_from(source)?)
                    .build(),
                "/modules",
            ))
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
        Mount::new_bind()
            .source((
                FromLayer::builder()
                    .from(PinnedReference::try_from(&module.get_module_image())?)
                    .build(),
                "/modules",
            ))
            .destination("/tmp/modules")
            .mode(MountMode::ReadWrite)
            .build()
    })
}

fn create_stage(
    recipe: &Recipe,
    context_dir: &Path,
    traversed_stages: &[&String],
    stage: &StageRequiredFields,
) -> Result<Option<Layer>, miette::Error> {
    if traversed_stages.contains(&&stage.name) {
        bail!("Hit cycle in build graph:\n{traversed_stages:?}");
    }
    let image = PinnedReference::try_from(&stage.from)?;
    let from = FromLayer::builder()
        .from(image)
        .maybe_platform(stage.platform)
        .build();
    Ok(Some(
        stage.get_processed_modules().iter().try_fold(
            from.into(),
            add_module(
                recipe,
                context_dir,
                once(&stage.name)
                    .chain(traversed_stages.iter().copied())
                    .collect(),
            ),
        )?,
    ))
}

#[cfg(test)]
mod test {
    use std::sync::LazyLock;

    use super::*;
    use rstest::rstest;

    static CONTEXT_DIR: LazyLock<&Path> = LazyLock::new(|| Path::new("./tests/repo/"));

    #[rstest]
    fn image() {
        let recipe = Recipe::builder()
            .path("./tests/repo/recipes/recipe.yml")
            .build()
            .unwrap();
        let image = Image::builder()
            .recipe(&recipe)
            .context_dir(&CONTEXT_DIR)
            .build()
            .unwrap();

        dbg!(&image);

        assert!(image.build().is_ok());
    }
}
