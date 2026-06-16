use std::{borrow::Cow, collections::HashMap};

use blue_build_utils::{
    constants::COSIGN_IMAGE_VERSION, container::Tag, env_str::EnvString, platform::Platform,
};
use bon::Builder;
use miette::{Context, IntoDiagnostic, Result};
use oci_client::Reference;
use serde::{Deserialize, Serialize};

use crate::{
    Module, ModuleExt, RecipeGetters, RecipeSetters, Stage, StagesExt, maybe_version::MaybeVersion,
};

/// The build recipe.
///
/// This is the top-level section of a recipe.yml.
/// This will contain information on the image and its
/// base image to assist with building the Containerfile
/// and tagging the image appropriately.
#[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]
#[allow(clippy::duplicated_attributes)]
#[builder(on(EnvString, into), on(String, into))]
pub struct RecipeV1 {
    /// The name of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.title` label.
    pub name: EnvString,

    /// The description of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.description` label.
    pub description: EnvString,

    /// The base image from which to build the user's image.
    #[serde(alias = "base-image")]
    pub base_image: EnvString,

    /// The version/tag of the base image.
    #[serde(rename = "image-version")]
    pub image_version: Tag,

    /// The version of `bluebuild` to install in the image
    #[serde(rename = "blue-build-tag", skip_serializing_if = "Option::is_none")]
    pub blue_build_tag: Option<MaybeVersion>,

    /// Alternate tags to the `latest` tag to add to the image.
    ///
    /// If `alt-tags` is not supplied by the user, the build system
    /// will assume `latest` and will also tag with the
    /// timestamp with no version (e.g. `20240429`).
    ///
    /// Any user input will override the `latest` and timestamp tags.
    #[serde(rename = "alt-tags", skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub alt_tags: Option<Vec<Tag>>,

    /// The version of nushell to use for modules.
    #[serde(skip_serializing_if = "Option::is_none", rename = "nushell-version")]
    pub nushell_version: Option<MaybeVersion>,

    /// The platforms to build for the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<Platform>>,

    /// The version of cosign to install.
    #[serde(skip_serializing_if = "Option::is_none", rename = "cosign-version")]
    pub cosign_version: Option<MaybeVersion>,

    /// The stages extension of the recipe.
    ///
    /// This hold the list of stages that can
    /// be used to build software outside of
    /// the final build image.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub stages_ext: Option<StagesExt>,

    /// The modules extension of the recipe.
    ///
    /// This holds the list of modules to be run on the image.
    #[serde(flatten)]
    pub modules_ext: ModuleExt,

    /// Custom LABELs to add to the image.
    ///
    /// This hashmap provides custom labels from ther use to the image
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub labels: Option<HashMap<String, EnvString>>,
}

impl RecipeGetters for RecipeV1 {
    fn get_modules(&self) -> &[Module] {
        &self.modules_ext.modules
    }

    fn get_stages(&self) -> &[Stage] {
        self.stages_ext.as_ref().map_or(&[], |ext| &ext.stages)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_description(&self) -> Option<&str> {
        Some(&self.description)
    }

    fn get_base_image(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.base_image)
    }

    fn base_image_ref(&self) -> Result<Reference> {
        let base_image = format!("{}:{}", self.base_image, self.image_version);
        base_image
            .parse()
            .into_diagnostic()
            .with_context(|| format!("Unable to parse base image {base_image}"))
    }

    fn get_labels(&self) -> HashMap<&str, &str> {
        self.labels
            .iter()
            .flatten()
            .map(|(key, value)| (&**key, &**value))
            .collect()
    }

    fn get_alt_tags(&self) -> Option<&[Tag]> {
        self.alt_tags.as_deref()
    }

    fn get_platforms(&self) -> &[Platform] {
        self.platforms.as_deref().unwrap_or(&[])
    }

    fn get_bluebuild_version(&self) -> Option<String> {
        match &self.blue_build_tag {
            None => Some("latest-installer".to_string()),
            Some(MaybeVersion::None) => None,
            Some(MaybeVersion::VersionOrBranch(ver)) => Some(format!("{ver}-installer")),
        }
    }

    fn get_cosign_version(&self) -> Option<String> {
        match &self.cosign_version {
            Some(MaybeVersion::None) => None,
            None => Some(format!("v{COSIGN_IMAGE_VERSION}")),
            Some(MaybeVersion::VersionOrBranch(version)) => Some(format!("v{version}")),
        }
    }

    fn get_nushell_version(&self) -> Option<String> {
        match &self.nushell_version {
            Some(MaybeVersion::None) => None,
            None => Some("default".to_string()),
            Some(MaybeVersion::VersionOrBranch(version)) => Some(version.to_string()),
        }
    }
}

impl RecipeSetters for RecipeV1 {
    fn set_modules(&mut self, modules: Vec<Module>) {
        self.modules_ext.modules = modules;
    }

    fn set_stages(&mut self, stages: Vec<Stage>) {
        if let Some(ext) = self.stages_ext.as_mut() {
            ext.stages = stages;
        } else {
            self.stages_ext = Some(StagesExt::builder().stages(stages).build());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn generate_test_recipe(
        custom_labels: HashMap<String, EnvString>,
    ) -> (BTreeMap<String, String>, RecipeV1) {
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
            RecipeV1::builder()
                .name("title".to_string())
                .description("description".to_string())
                .base_image("base_name".to_string())
                .image_version("42".parse().unwrap())
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
            Some(&"build_id".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.title"),
            Some(&"title".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.description"),
            Some(&"description".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.source"),
            Some(&"source".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.base.digest"),
            Some(&"digest".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.base.name"),
            Some(&"base_name".into())
        );
        assert_eq!(
            labels.get("org.opencontainers.image.created"),
            Some(&"today 15:30".into())
        );
        assert_eq!(
            labels.get("io.artifacthub.package.readme-url"),
            Some(&"https://raw.githubusercontent.com/blue-build/cli/main/README.md".into())
        );

        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn test_custom_label_overwrite_generation() {
        let custom_labels = HashMap::from([(
            "io.artifacthub.package.readme-url".into(),
            "https://test.html".into(),
        )]);
        let (built_in_labels, recipe) = generate_test_recipe(custom_labels);
        let labels = recipe.generate_labels(&built_in_labels);

        assert_eq!(
            labels.get("io.artifacthub.package.readme-url"),
            Some(&"https://test.html".into())
        );
        assert_eq!(labels.len(), 8);
    }

    #[test]
    fn test_custom_label_addition_generation() {
        let custom_labels = HashMap::from([("org.container.test".into(), "test1".into())]);
        let (built_in_labels, recipe) = generate_test_recipe(custom_labels);
        let labels = recipe.generate_labels(&built_in_labels);

        assert_eq!(labels.get("org.container.test"), Some(&"test1".into()));
        assert_eq!(labels.len(), 9);
    }
}
