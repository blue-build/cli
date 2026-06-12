use std::{
    fs,
    path::{Path, PathBuf},
};

use bon::Builder;
use miette::{Context, IntoDiagnostic, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{FromFileList, Module, base_recipe_path};

#[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]
pub struct ModuleExt {
    #[builder(default)]
    pub modules: Vec<Module>,
}

impl FromFileList for ModuleExt {
    const LIST_KEY: &'static str = "modules";

    fn get_from_file_paths(&self) -> Vec<PathBuf> {
        self.modules
            .iter()
            .filter_map(Module::get_from_file_path)
            .collect()
    }
}

impl TryFrom<&PathBuf> for ModuleExt {
    type Error = Report;

    fn try_from(value: &PathBuf) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_path())
    }
}

impl TryFrom<&Path> for ModuleExt {
    type Error = Report;

    fn try_from(file_name: &Path) -> Result<Self> {
        let file_path = base_recipe_path().join(file_name);

        let file = fs::read_to_string(&file_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open {}", file_path.display()))?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let module = serde_yaml::from_str::<Module>(&file)
                    .into_diagnostic()
                    .wrap_err_with(|| {
                        format!("Failed to parse module file {}", file_path.display())
                    })?;
                Ok(Self::builder().modules(vec![module]).build())
            },
            Ok,
        )
    }
}
