//! This library is the Next Generation Build Engine for BlueBuild.
//!
//! This build engine is based on `buildah` and allows building a
//! directed acyclical graph (DAG) from a BlueBuild recipe. This
//! engine takes advantage of OCI per-layer operations
//! that allow us  evaluate the state of the image mid-build that a
//! standard `Containerfile` is not capable of.
#![doc = include_str!("../LGPL_2.1.txt")]

pub(crate) mod containers;
pub mod layer;
pub mod path;
pub mod reference;

use std::{
    collections::HashMap, iter::once, path::Path, process::Command, str::FromStr, sync::Arc,
};

use blue_build_recipe::{ModuleRequiredFields, Recipe, RecipeGetters, StageRequiredFields};
use blue_build_utils::platform::Platform;
use miette::{IntoDiagnostic, Result, bail};
use oci_client::Reference;
use rayon::prelude::*;

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
    pub fn new(recipe: &Recipe, squash: bool) -> Result<Self> {
        let image = recipe
            .base_image_ref()
            .and_then(|image| PinnedReference::pin_image(&image))?;
        let platforms = recipe.get_platforms();
        // let build_scripts = BuildScripts::extract_mount_dir()?;
        let create_image = |platform| {
            let image_plan = Image::builder()
                .image(image.clone())
                .platform(platform)
                .modules(&recipe.get_processed_modules())
                .stages(&recipe.get_processed_stages())
                .squash(squash)
                .context_dir(Path::new("."))
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
    pub fn build(self) -> Result<HashMap<Platform, Arc<LayerId>>> {
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
        image: Arc<PinnedReference>,
        platform: Platform,
        modules: &[&ModuleRequiredFields],
        stages: &[&StageRequiredFields],
        squash: bool,
        context_dir: &Path,
    ) -> Result<Self> {
        let from = FromLayer::builder()
            .from(image)
            .platform(platform)
            .squash(squash)
            .build();
        let layers = modules
            .iter()
            .try_fold(from.into(), add_module(context_dir, stages, Vec::new()))?;
        Ok(Self { layers })
    }

    /// Build the image.
    ///
    /// # Errors
    /// Will error if the build fails.
    pub fn build(self) -> Result<Arc<LayerId>> {
        self.layers.finalize()
    }
}

fn add_module(
    context_dir: &Path,
    stages: &[&StageRequiredFields],
    traversed_stages: Vec<&String>,
) -> impl FnMut(Layer, &&ModuleRequiredFields) -> Result<Layer> {
    move |parent, module| {
        Ok(match module.module_type.typ() {
            "copy" => {
                let source = module.config["src"].as_str().unwrap();
                let destination = module.config["dest"].as_str().unwrap();
                let from = if let Some(stage) = module.config["from"]
                    .as_str()
                    .and_then(|from| stages.iter().find(|stage| stage.name == from))
                {
                    create_stage(context_dir, stages, &traversed_stages, stage)?
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
                let cmd = Command::try_from(*module)?;
                let modules_mnt = modules_mnt(context_dir, module)?;
                let files_mnt = files_mnt(context_dir)?;
                let nushell_mnt = nushell_mnt()?;

                RunLayer::builder()
                    .parent(parent)
                    .cmd(cmd)
                    .mounts([modules_mnt, files_mnt, nushell_mnt])
                    .build()
                    .into()
            }
        })
    }
}

fn nushell_mnt() -> Result<Mount> {
    let image = PinnedReference::pin_image(
        &format!("{}:default", blue_build_utils::constants::NUSHELL_IMAGE)
            .parse()
            .into_diagnostic()?,
    )?;
    Ok(Mount::new_bind()
        .source((image, "/nu"))
        .destination("/usr/libexec/bluebuild/nu")
        .build())
}

fn files_mnt(context_dir: &Path) -> Result<Mount, miette::Error> {
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

fn modules_mnt(context_dir: &Path, module: &ModuleRequiredFields) -> Result<Mount> {
    Ok(if let Some(source) = module.get_non_local_source() {
        let image = PinnedReference::pin_image(&source.parse().into_diagnostic()?)?;
        Mount::new_bind()
            .source((image, "/modules"))
            .destination("/tmp/modules")
            .mode(MountMode::ReadWrite)
            .build()
    } else if module.is_local_source() {
        let modules = CopyLayer::builder()
            .parent(FromLayer::builder().build())
            .source(context_dir, "./modules")?
            .destination("/modules")
            .build();

        Mount::new_bind()
            .source((modules, "/modules"))
            .destination("/tmp/modules")
            .mode(MountMode::Default)
            .build()
    } else {
        let image = module
            .get_module_image()
            .parse::<Reference>()
            .into_diagnostic()
            .and_then(|image| PinnedReference::pin_image(&image))?;
        Mount::new_bind()
            .source((image, "/modules"))
            .destination("/tmp/modules")
            .mode(MountMode::ReadWrite)
            .build()
    })
}

fn create_stage(
    context_dir: &Path,
    stages: &[&StageRequiredFields],
    traversed_stages: &[&String],
    stage: &StageRequiredFields,
) -> Result<Option<Layer>, miette::Error> {
    if traversed_stages.contains(&&stage.name) {
        bail!("Hit cycle in build graph:\n{traversed_stages:?}");
    }
    let image = PinnedReference::pin_image(&Reference::from_str(&stage.from).into_diagnostic()?)?;
    let from = FromLayer::builder()
        .from(image)
        .maybe_platform(stage.platform)
        .build();
    Ok(Some(
        stage.get_processed_modules().iter().try_fold(
            from.into(),
            add_module(
                context_dir,
                stages,
                once(&stage.name)
                    .chain(traversed_stages.iter().copied())
                    .collect(),
            ),
        )?,
    ))
}
