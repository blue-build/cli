use std::{
    borrow::Cow,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use blue_build_utils::secret::Secret;
use bon::Builder;
use cached::proc_macro::cached;
use log::{debug, trace};
use miette::{Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleExt, StagesExt, maybe_version::MaybeVersion};

/// The build recipe.
///
/// This is the top-level section of a recipe.yml.
/// This will contain information on the image and its
/// base image to assist with building the Containerfile
/// and tagging the image appropriately.
#[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]
pub struct Recipe<'a> {
    /// The name of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.title` label.
    #[builder(into)]
    pub name: Cow<'a, str>,

    /// The description of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.description` label.
    #[builder(into)]
    pub description: Cow<'a, str>,

    /// The base image from which to build the user's image.
    #[serde(alias = "base-image")]
    #[builder(into)]
    pub base_image: Cow<'a, str>,

    /// The version/tag of the base image.
    #[serde(alias = "image-version")]
    #[builder(into)]
    pub image_version: Cow<'a, str>,

    /// The version of `bluebuild` to install in the image
    #[serde(alias = "blue-build-tag", skip_serializing_if = "Option::is_none")]
    pub blue_build_tag: Option<MaybeVersion>,

    /// Alternate tags to the `latest` tag to add to the image.
    ///
    /// If `alt-tags` is not supplied by the user, the build system
    /// will assume `latest` and will also tag with the
    /// timestamp with no version (e.g. `20240429`).
    ///
    /// Any user input will override the `latest` and timestamp tags.
    #[serde(alias = "alt-tags", skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub alt_tags: Option<Vec<String>>,

    /// The version of nushell to use for modules.
    #[serde(skip_serializing_if = "Option::is_none", rename = "nushell-version")]
    pub nushell_version: Option<MaybeVersion>,

    /// The stages extension of the recipe.
    ///
    /// This hold the list of stages that can
    /// be used to build software outside of
    /// the final build image.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub stages_ext: Option<StagesExt<'a>>,

    /// The modules extension of the recipe.
    ///
    /// This holds the list of modules to be run on the image.
    #[serde(flatten)]
    pub modules_ext: ModuleExt<'a>,
}

impl Recipe<'_> {
    /// Parse a recipe file
    ///
    /// # Errors
    /// Errors when a yaml file cannot be deserialized,
    /// or a linked module yaml file does not exist.
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[cached(result = true, key = "PathBuf", convert = r"{ path.into() }")]
        fn inner(path: &Path) -> Result<Recipe<'static>> {
            trace!("Recipe::parse({})", path.display());

            let file_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir().into_diagnostic()?.join(path)
            };

            let file = fs::read_to_string(&file_path)
                .into_diagnostic()
                .with_context(|| format!("Failed to read {}", file_path.display()))?;

            debug!("Recipe contents: {file}");

            let mut recipe = serde_yaml::from_str::<Recipe>(&file)
                .map_err(blue_build_utils::serde_yaml_err(&file))
                .into_diagnostic()?;

            recipe.modules_ext.modules = Module::get_modules(&recipe.modules_ext.modules, None)?;

            if let Some(ref mut stages_ext) = recipe.stages_ext {
                stages_ext.stages = crate::Stage::get_stages(&stages_ext.stages, None)?;
            }

            Ok(recipe)
        }
        inner(path.as_ref())
    }

    /// Get a `Reference` object of the `base_image`.
    ///
    /// # Errors
    /// Will error if it fails to parse the `base_image`.
    pub fn base_image_ref(&self) -> Result<Reference> {
        let base_image = format!("{}:{}", self.base_image, self.image_version);
        base_image
            .parse()
            .into_diagnostic()
            .with_context(|| format!("Unable to parse base image {base_image}"))
    }

    #[must_use]
    pub const fn should_install_bluebuild(&self) -> bool {
        match self.blue_build_tag {
            None | Some(MaybeVersion::Version(_)) => true,
            Some(MaybeVersion::None) => false,
        }
    }

    #[must_use]
    pub fn get_bluebuild_version(&self) -> String {
        match &self.blue_build_tag {
            Some(MaybeVersion::None) | None => "latest-installer".to_string(),
            Some(MaybeVersion::Version(version)) => version.to_string(),
        }
    }

    #[must_use]
    pub fn get_secrets(&self) -> Vec<&Secret> {
        self.modules_ext
            .modules
            .iter()
            .filter_map(|module| Some(&module.required_fields.as_ref()?.secrets))
            .flatten()
            .chain(
                self.stages_ext
                    .as_ref()
                    .map_or_else(Vec::new, |stage| stage.stages.iter().collect())
                    .iter()
                    .filter_map(|stage| Some(&stage.required_fields.as_ref()?.modules_ext.modules))
                    .flatten()
                    .filter_map(|module| Some(&module.required_fields.as_ref()?.secrets))
                    .flatten(),
            )
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}
