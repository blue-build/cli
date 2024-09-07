use std::{borrow::Cow, fs, path::Path};

use blue_build_utils::cowstr;
use indexmap::IndexMap;
use log::{debug, trace};
use miette::{Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::{Module, ModuleExt, StagesExt};

/// The build recipe.
///
/// This is the top-level section of a recipe.yml.
/// This will contain information on the image and its
/// base image to assist with building the Containerfile
/// and tagging the image appropriately.
#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct Recipe<'a> {
    /// The name of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.title` label.
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    /// The description of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.description` label.
    #[builder(setter(into))]
    pub description: Cow<'a, str>,

    /// The base image from which to build the user's image.
    #[serde(alias = "base-image")]
    #[builder(setter(into))]
    pub base_image: Cow<'a, str>,

    /// The version/tag of the base image.
    #[serde(alias = "image-version")]
    #[builder(setter(into))]
    pub image_version: Cow<'a, str>,

    /// The version of `bluebuild` to install in the image
    #[serde(alias = "blue-build-tag", skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into, strip_option))]
    pub blue_build_tag: Option<Cow<'a, str>>,

    /// Alternate tags to the `latest` tag to add to the image.
    ///
    /// If `alt-tags` is not supplied by the user, the build system
    /// will assume `latest` and will also tag with the
    /// timestamp with no version (e.g. `20240429`).
    ///
    /// Any user input will override the `latest` and timestamp tags.
    #[serde(alias = "alt-tags", skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into, strip_option))]
    alt_tags: Option<Vec<Cow<'a, str>>>,

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

    /// Extra data that the user might have added. This is
    /// done in case we serialize the data to a yaml file
    /// so that we retain any unused information.
    #[serde(flatten)]
    #[builder(setter(into))]
    pub extra: IndexMap<String, Value>,
}

impl<'a> Recipe<'a> {
    /// Parse a recipe file
    ///
    /// # Errors
    /// Errors when a yaml file cannot be deserialized,
    /// or a linked module yaml file does not exist.
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self> {
        trace!("Recipe::parse({})", path.as_ref().display());

        let file_path = if Path::new(path.as_ref()).is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            std::env::current_dir()
                .into_diagnostic()?
                .join(path.as_ref())
        };

        let file = fs::read_to_string(&file_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to read {}", file_path.display()))?;

        debug!("Recipe contents: {file}");

        let mut recipe = serde_yaml::from_str::<Recipe>(&file)
            .map_err(blue_build_utils::serde_yaml_err(&file))
            .into_diagnostic()?;

        recipe.modules_ext.modules = Module::get_modules(&recipe.modules_ext.modules, None)?.into();

        #[cfg(feature = "stages")]
        if let Some(ref mut stages_ext) = recipe.stages_ext {
            stages_ext.stages = crate::Stage::get_stages(&stages_ext.stages, None)?.into();
        }

        #[cfg(not(feature = "stages"))]
        {
            recipe.stages_ext = None;
        }

        Ok(recipe)
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
    pub fn alt_tags(&'a self) -> Option<Vec<Cow<'a, str>>> {
        self.alt_tags
            .as_ref()
            .map(|tags| tags.iter().map(|tag| cowstr!(&**tag)).collect())
    }
}
