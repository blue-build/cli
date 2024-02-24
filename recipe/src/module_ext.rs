use std::{borrow::Cow, collections::HashSet, fs, path::PathBuf};

use anyhow::Result;
use log::trace;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{AkmodsInfo, Module};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct ModuleExt<'a> {
    #[builder(default, setter(into))]
    pub modules: Cow<'a, [Module<'a>]>,
}

impl ModuleExt<'_> {
    /// # Parse a module file returning a [`ModuleExt`]
    ///
    /// # Errors
    /// Can return an `anyhow` Error if the file cannot be read or deserialized
    /// into a [`ModuleExt`]
    pub fn parse_module_from_file(file_name: &str) -> Result<Self> {
        let file_path = PathBuf::from("config").join(file_name);
        let file_path = if file_path.is_absolute() {
            file_path
        } else {
            std::env::current_dir()?.join(file_path)
        };

        let file = fs::read_to_string(file_path)?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let module = serde_yaml::from_str::<Module>(&file)
                    .map_err(blue_build_utils::serde_yaml_err(&file))?;
                Ok(Self::builder().modules(vec![module]).build())
            },
            Ok,
        )
    }

    pub fn get_akmods_info_list(&self, os_version: &str) -> Vec<AkmodsInfo> {
        trace!("get_akmods_image_list({self:#?}, {os_version})");

        let mut seen = HashSet::new();

        self.modules
            .iter()
            .filter(|module| module.module_type.as_ref().is_some_and(|t| t == "akmods"))
            .map(|module| module.generate_akmods_info(os_version))
            .filter(|image| seen.insert(image.clone()))
            .collect()
    }
}
