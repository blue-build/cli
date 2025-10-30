use std::{
    borrow::Cow,
    collections::HashSet,
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use blue_build_utils::secret::Secret;
use bon::Builder;
use cached::proc_macro::cached;
use log::{debug, trace, warn};
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

    /// Custom LABELs to add to the image.
    ///
    /// This hashmap provides custom labels from ther use to the image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
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
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to parse recipe file {}", file_path.display()))?;

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

    #[must_use]
    pub fn generate_labels(
        &self,
        default_labels: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        #[allow(clippy::option_if_let_else)] // map_or_else won't work with returning ref
        let labels = if let Some(labels) = &self.labels {
            labels
        } else {
            &HashMap::new()
        };

        let mut labels = default_labels.iter().chain(labels).fold(
            BTreeMap::new(),
            |mut acc, (k, v)| {
                if let Some(existing_value) = acc.get(k) {
                    warn!("Found conflicting values for label: {k}, contains: {existing_value}, overwritten by: {v}");
                }
                acc.insert(k.clone(), v.clone());
                acc
            },
        );

        if !labels.contains_key("io.artifacthub.package.readme-url") {
            // adding this if not included in the custom labeling to maintain backwards compatibility since this was hardcoded into the old template
            labels.insert(
                "io.artifacthub.package.readme-url".to_string(),
                "https://raw.githubusercontent.com/blue-build/cli/main/README.md".to_string(),
            );
        }

        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_recipe(
        custom_labels: HashMap<String, String>,
    ) -> (BTreeMap<String, String>, Recipe<'static>) {
        let default_labels = BTreeMap::from([
            (
                blue_build_utils::constants::BUILD_ID_LABEL.to_string(),
                "build_id".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "title".to_string(),
            ),
            (
                "org.opencontainers.image.description".to_string(),
                "description".to_string(),
            ),
            (
                "org.opencontainers.image.source".to_string(),
                "source".to_string(),
            ),
            (
                "org.opencontainers.image.base.digest".to_string(),
                "digest".to_string(),
            ),
            (
                "org.opencontainers.image.base.name".to_string(),
                "base_name".to_string(),
            ),
            (
                "org.opencontainers.image.created".to_string(),
                "today 15:30".to_string(),
            ),
        ]);
        (
            default_labels,
            Recipe::builder()
                .name("title".to_string())
                .description("description".to_string())
                .base_image("base_name".to_string())
                .image_version("version".to_string())
                .modules_ext(ModuleExt::builder().modules(vec![]).build())
                .labels(custom_labels)
                .build(),
        )
    }
    #[test]
    fn test_default_label_generation() {
        let custom_labels = HashMap::new();
        let (built_in_labels, recipe) = generate_test_recipe(custom_labels);
        let labels = recipe.generate_labels(&built_in_labels);
        assert_eq!(
            labels.get(blue_build_utils::constants::BUILD_ID_LABEL),
            Some(&"build_id".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.title"),
            Some(&"title".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.description"),
            Some(&"description".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.source"),
            Some(&"source".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.base.digest"),
            Some(&"digest".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.base.name"),
            Some(&"base_name".to_string())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.created"),
            Some(&"today 15:30".to_string())
        );
        assert_eq!(
            labels.get("io.artifacthub.package.readme-url"),
            Some(&"https://raw.githubusercontent.com/blue-build/cli/main/README.md".to_string())
        );

        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn test_custom_label_overwrite_generation() {
        let custom_labels = HashMap::from([(
            "io.artifacthub.package.readme-url".to_string(),
            "https://test.html".to_string(),
        )]);
        let (built_in_labels, recipe) = generate_test_recipe(custom_labels);
        let labels = recipe.generate_labels(&built_in_labels);

        assert_eq!(
            labels.get("io.artifacthub.package.readme-url"),
            Some(&"https://test.html".to_string())
        );
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn test_custom_label_addition_generation() {
        let custom_labels =
            HashMap::from([("org.container.test".to_string(), "test1".to_string())]);
        let (built_in_labels, recipe) = generate_test_recipe(custom_labels);
        let labels = recipe.generate_labels(&built_in_labels);

        assert_eq!(labels.get("org.container.test"), Some(&"test1".to_string()));
        assert_eq!(labels.len(), 9);
    }
}
