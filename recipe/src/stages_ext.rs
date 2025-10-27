use std::{
    fs,
    path::{Path, PathBuf},
};

use bon::Builder;
use miette::{Context, IntoDiagnostic, Report, Result};
use serde::{Deserialize, Serialize};

use crate::{FromFileList, Module, Stage, base_recipe_path};

#[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]
pub struct StagesExt<'a> {
    #[builder(default)]
    pub stages: Vec<Stage<'a>>,
}

impl FromFileList for StagesExt<'_> {
    const LIST_KEY: &'static str = "stages";

    fn get_from_file_paths(&self) -> Vec<PathBuf> {
        self.stages
            .iter()
            .filter_map(Stage::get_from_file_path)
            .collect()
    }

    fn get_module_from_file_paths(&self) -> Vec<PathBuf> {
        self.stages
            .iter()
            .flat_map(|stage| {
                stage
                    .required_fields
                    .as_ref()
                    .map_or_else(Vec::new, |rf| rf.modules_ext.get_from_file_paths())
            })
            .collect()
    }
}

impl TryFrom<&PathBuf> for StagesExt<'_> {
    type Error = Report;

    fn try_from(value: &PathBuf) -> Result<Self> {
        Self::try_from(value.as_path())
    }
}

impl TryFrom<&Path> for StagesExt<'_> {
    type Error = Report;

    fn try_from(file_name: &Path) -> Result<Self> {
        let file_path = base_recipe_path().join(file_name);

        let file = fs::read_to_string(&file_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open {}", file_path.display()))?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let mut stage = serde_yaml::from_str::<Stage>(&file)
                    .into_diagnostic()
                    .wrap_err_with(|| {
                        format!("Failed to parse stage file {}", file_path.display())
                    })?;
                if let Some(ref mut rf) = stage.required_fields {
                    rf.modules_ext.modules = Module::get_modules(&rf.modules_ext.modules, None)?;
                }
                Ok(Self::builder().stages(vec![stage]).build())
            },
            |mut stages_ext| -> Result<Self> {
                let mut stages: Vec<Stage> =
                    stages_ext.stages.iter().map(ToOwned::to_owned).collect();
                for stage in &mut stages {
                    if let Some(ref mut rf) = stage.required_fields {
                        rf.modules_ext.modules =
                            Module::get_modules(&rf.modules_ext.modules, None)?;
                    }
                }
                stages_ext.stages = stages;
                Ok(stages_ext)
            },
        )
    }
}
