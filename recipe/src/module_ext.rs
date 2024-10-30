use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use bon::Builder;
use log::trace;
use miette::{Context, IntoDiagnostic, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{base_recipe_path, AkmodsInfo, FromFileList, Module};

#[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]
pub struct ModuleExt<'a> {
    #[builder(default)]
    pub modules: Vec<Module<'a>>,
}

impl FromFileList for ModuleExt<'_> {
    const LIST_KEY: &'static str = "modules";

    #[must_use]
    fn get_from_file_paths(&self) -> Vec<PathBuf> {
        self.modules
            .iter()
            .filter_map(Module::get_from_file_path)
            .collect()
    }
}

impl TryFrom<&PathBuf> for ModuleExt<'_> {
    type Error = Report;

    fn try_from(value: &PathBuf) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_path())
    }
}

impl TryFrom<&Path> for ModuleExt<'_> {
    type Error = Report;

    fn try_from(file_name: &Path) -> Result<Self> {
        let file_path = base_recipe_path().join(file_name);

        let file = fs::read_to_string(&file_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open {}", file_path.display()))?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let module = serde_yaml::from_str::<Module>(&file)
                    .map_err(blue_build_utils::serde_yaml_err(&file))
                    .into_diagnostic()?;
                Ok(Self::builder().modules(vec![module]).build())
            },
            Ok,
        )
    }
}

impl ModuleExt<'_> {
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
