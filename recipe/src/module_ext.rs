use std::{borrow::Cow, collections::HashSet, fs, path::Path};

use anyhow::{Context, Result};
use blue_build_utils::constants::{CONFIG_PATH, RECIPE_PATH};
use log::{trace, warn};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{AkmodsInfo, Module};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct ModuleExt<'a> {
    #[builder(default, setter(into))]
    pub modules: Cow<'a, [Module<'a>]>,
}

impl ModuleExt<'_> {
    /// Parse a module file returning a [`ModuleExt`]
    ///
    /// # Errors
    /// Can return an `anyhow` Error if the file cannot be read or deserialized
    /// into a [`ModuleExt`]
    pub fn parse(file_name: &Path) -> Result<Self> {
        let legacy_path = Path::new(CONFIG_PATH);
        let recipe_path = Path::new(RECIPE_PATH);

        let file_path = if recipe_path.exists() && recipe_path.is_dir() {
            recipe_path.join(file_name)
        } else {
            warn!("Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}");
            legacy_path.join(file_name)
        };

        let file = fs::read_to_string(&file_path)
            .context(format!("Failed to open {}", file_path.display()))?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let module = serde_yaml::from_str::<Module>(&file)
                    .map_err(blue_build_utils::serde_yaml_err(&file))?;
                Ok(Self::builder().modules(vec![module]).build())
            },
            Ok,
        )
    }

    #[must_use]
    pub fn get_akmods_info_list(&self, os_version: &u64) -> Vec<AkmodsInfo> {
        trace!("get_akmods_image_list({self:#?}, {os_version})");

        let mut seen = HashSet::new();

        self.modules
            .iter()
            .filter(|module| {
                module
                    .required_fields
                    .as_ref()
                    .is_some_and(|rf| rf.module_type == "akmods")
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
