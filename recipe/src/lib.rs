use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use blue_build_utils::{
    constants::{CONFIG_PATH, RECIPE_PATH},
    container::Tag,
    platform::Platform,
    secret::Secret,
};
use cached::cached;
use log::{debug, trace, warn};
use miette::{Context, IntoDiagnostic, Result};
use oci_client::Reference;
use serde::{Deserialize, Serialize};
use serde_yaml::{Number, Value};

mod akmods_info;
mod maybe_version;
mod module;
mod module_ext;
mod recipe_v1;
mod recipe_v2;
mod stage;
mod stages_ext;

pub use akmods_info::*;
pub use maybe_version::*;
pub use module::*;
pub use module_ext::*;
pub use recipe_v1::*;
#[cfg(feature = "recipe-v2")]
pub use recipe_v2::*;
pub use stage::*;
pub use stages_ext::*;

pub trait FromFileList {
    const LIST_KEY: &str;

    fn get_from_file_paths(&self) -> Vec<PathBuf>;

    fn get_module_from_file_paths(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

trait RecipeSetters: RecipeGetters {
    fn process_from_files(&mut self) -> Result<()> {
        self.set_modules(Module::get_modules(self.get_modules(), None)?);
        self.set_stages(Stage::get_stages(self.get_stages(), None)?);
        Ok(())
    }

    fn set_modules(&mut self, modules: Vec<Module>);

    fn set_stages(&mut self, stages: Vec<Stage>);
}

pub trait RecipeGetters {
    fn get_name(&self) -> &str;
    fn get_description(&self) -> Option<&str>;
    fn get_modules(&self) -> &[Module];
    fn get_stages(&self) -> &[Stage];
    fn get_labels(&self) -> HashMap<&str, &str>;
    fn get_alt_tags(&self) -> Option<&[Tag]>;
    fn get_platforms(&self) -> &[Platform];
    fn get_base_image(&self) -> Cow<'_, str>;
    fn get_bluebuild_version(&self) -> Option<String>;
    fn get_cosign_version(&self) -> Option<String>;
    fn get_nushell_version(&self) -> Option<String>;

    /// Get the base image reference.
    ///
    /// # Errors
    /// Will error if the reference isn't valid.
    fn base_image_ref(&self) -> Result<Reference>;

    fn should_install_bins(&self) -> bool {
        self.get_bluebuild_version().is_some() || self.get_cosign_version().is_some()
    }

    fn generate_labels(
        &self,
        default_labels: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut labels = default_labels.iter().map(|(key, value)| (&**key, &**value)).chain(self.get_labels()).fold(
            BTreeMap::new(),
            |mut acc, (k, v)| {
                if let Some(existing_value) = acc.get(k) {
                    warn!("Found conflicting values for label: {k}, contains: {existing_value}, overwritten by: {v}");
                }
                acc.insert(k.to_owned(), v.to_owned());
                acc
            },
        );

        if !labels.contains_key("io.artifacthub.package.readme-url") {
            // adding this if not included in the custom labeling to maintain backwards compatibility since this was hardcoded into the old template
            labels.insert(
                "io.artifacthub.package.readme-url".into(),
                "https://raw.githubusercontent.com/blue-build/cli/main/README.md".into(),
            );
        }

        labels
    }

    fn get_secrets(&self) -> Vec<&Secret> {
        self.get_modules()
            .iter()
            .filter_map(|module| Some(&module.required_fields.as_ref()?.secrets))
            .flatten()
            .chain(
                self.get_stages()
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

    fn get_processed_modules(&self) -> Vec<&ModuleRequiredFields> {
        self.get_modules()
            .iter()
            .filter_map(|module| module.required_fields.as_ref())
            .collect()
    }

    fn get_processed_stages(&self) -> Vec<&StageRequiredFields> {
        self.get_stages()
            .iter()
            .filter_map(|stage| stage.required_fields.as_ref())
            .collect()
    }

    fn get_akmods_info_list(&self, os_version: &u64) -> Vec<AkmodsInfo> {
        trace!("get_akmods_image_list({os_version})");

        let mut seen = HashSet::new();

        self.get_modules()
            .iter()
            .filter(|module| {
                module
                    .required_fields
                    .as_ref()
                    .is_some_and(|rf| rf.module_type.typ() == "akmods")
            })
            .filter_map(|module| {
                Some(
                    module
                        .required_fields
                        .as_ref()?
                        .generate_akmods_info(os_version),
                )
            })
            .filter(|image| seen.insert(image.clone()))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub enum Recipe {
    V1(Box<RecipeV1>),
    #[cfg(feature = "recipe-v2")]
    V2(Box<RecipeV2>),
}

impl Recipe {
    /// Parse a recipe file
    ///
    /// # Errors
    /// Errors when a yaml file cannot be deserialized,
    /// or a linked module yaml file does not exist.
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self> {
        #[cached(key = "PathBuf", convert = r"{ path.into() }")]
        fn inner(path: &Path) -> Result<Recipe> {
            trace!("Recipe::parse({})", path.display());

            #[cfg(not(test))]
            let file_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir().into_diagnostic()?.join(path)
            };

            #[cfg(test)]
            let file_path = Path::new(test::REPO_PATH).join(path);

            let file = fs::read_to_string(&file_path)
                .into_diagnostic()
                .with_context(|| format!("Failed to read {}", file_path.display()))?;

            debug!("Recipe contents: {file}");

            let mut recipe = serde_yaml::from_str::<Recipe>(&file)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to parse recipe file {}", file_path.display()))?;
            recipe.process_from_files()?;
            Ok(recipe)
        }
        inner(path.as_ref())
    }

    #[must_use]
    #[cfg_attr(not(feature = "recipe-v2"), expect(clippy::missing_const_for_fn))]
    pub fn upgrade(self) -> Self {
        match self {
            #[cfg(feature = "recipe-v2")]
            Self::V1(recipe) => Self::V2(Box::new(RecipeV2::from(*recipe))),

            #[cfg(feature = "recipe-v2")]
            recipe @ Self::V2(_) => recipe,

            #[cfg(not(feature = "recipe-v2"))]
            recipe @ Self::V1(_) => recipe,
        }
    }

    fn version_number(&self) -> Number {
        match self {
            Self::V1(_) => 1,
            #[cfg(feature = "recipe-v2")]
            Self::V2(_) => 2,
        }
        .into()
    }
}

impl<'de> Deserialize<'de> for Recipe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let obj = Value::deserialize(deserializer)?;

        Ok(match obj["version"].as_u64() {
            Some(1) | None => Self::V1(Box::new(
                serde_yaml::from_value(obj).map_err(serde::de::Error::custom)?,
            )),
            #[cfg(feature = "recipe-v2")]
            Some(2) => Self::V2(Box::new(
                serde_yaml::from_value(obj).map_err(serde::de::Error::custom)?,
            )),
            Some(version) => {
                return Err(serde::de::Error::custom(format!(
                    "Unexpected recipe version {version}"
                )));
            }
        })
    }
}

impl Serialize for Recipe {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let version = Value::Number(self.version_number());
        let mut obj = match self {
            Self::V1(recipe) => serde_yaml::to_value(recipe),
            #[cfg(feature = "recipe-v2")]
            Self::V2(recipe) => serde_yaml::to_value(recipe),
        }
        .map_err(serde::ser::Error::custom)?;
        obj["version"] = version;
        obj.serialize(serializer)
    }
}

macro_rules! impl_recipe {
    ($self:ident, $func:ident($($args:expr),*)) => {
        match $self {
            Self::V1(recipe) => recipe.$func($($args,)*),
            #[cfg(feature = "recipe-v2")]
            Self::V2(recipe) => recipe.$func($($args,)*),
        }
    };
}

impl RecipeGetters for Recipe {
    fn get_name(&self) -> &str {
        impl_recipe!(self, get_name())
    }

    fn get_description(&self) -> Option<&str> {
        impl_recipe!(self, get_description())
    }

    fn get_modules(&self) -> &[Module] {
        impl_recipe!(self, get_modules())
    }

    fn get_stages(&self) -> &[Stage] {
        impl_recipe!(self, get_stages())
    }

    fn get_labels(&self) -> HashMap<&str, &str> {
        impl_recipe!(self, get_labels())
    }

    fn base_image_ref(&self) -> Result<Reference> {
        impl_recipe!(self, base_image_ref())
    }

    fn get_alt_tags(&self) -> Option<&[Tag]> {
        impl_recipe!(self, get_alt_tags())
    }

    fn get_platforms(&self) -> &[Platform] {
        impl_recipe!(self, get_platforms())
    }

    fn get_base_image(&self) -> Cow<'_, str> {
        impl_recipe!(self, get_base_image())
    }

    fn get_bluebuild_version(&self) -> Option<String> {
        impl_recipe!(self, get_bluebuild_version())
    }

    fn get_cosign_version(&self) -> Option<String> {
        impl_recipe!(self, get_cosign_version())
    }

    fn get_nushell_version(&self) -> Option<String> {
        impl_recipe!(self, get_nushell_version())
    }
}

impl RecipeSetters for Recipe {
    fn set_modules(&mut self, modules: Vec<Module>) {
        impl_recipe!(self, set_modules(modules));
    }

    fn set_stages(&mut self, stages: Vec<Stage>) {
        impl_recipe!(self, set_stages(stages));
    }
}

impl Default for Recipe {
    fn default() -> Self {
        #[cfg(feature = "recipe-v2")]
        {
            Self::V2(RecipeV2::default().into())
        }
        #[cfg(not(feature = "recipe-v2"))]
        {
            Self::V1(RecipeV1::default().into())
        }
    }
}

pub(crate) fn base_recipe_path() -> PathBuf {
    #[cfg(not(test))]
    let (legacy_path, recipe_path) = (PathBuf::from(CONFIG_PATH), PathBuf::from(RECIPE_PATH));

    #[cfg(test)]
    let (legacy_path, recipe_path) = (
        Path::new(crate::test::REPO_PATH).join(CONFIG_PATH),
        Path::new(crate::test::REPO_PATH).join(RECIPE_PATH),
    );

    if recipe_path.exists() && recipe_path.is_dir() {
        recipe_path
    } else {
        warn!(
            "Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}"
        );
        legacy_path
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::Recipe;

    pub const REPO_PATH: &str = "../integration-tests/test-repo";

    #[rstest]
    #[case::recipe_v1("recipes/recipe.yml")]
    #[cfg_attr(feature = "recipe-v2", case::recipe_v2("recipes/recipe-v2.yml"))]
    fn parse_recipe(#[case] recipe_path: &str) {
        // serialize
        let recipe = Recipe::parse(recipe_path).unwrap();

        // deserialize
        serde_yaml::to_string(&recipe).unwrap();
    }
}
