use std::{borrow::Cow, fs, path::Path};

use anyhow::{Context, Result};
use blue_build_utils::constants::{CONFIG_PATH, RECIPE_PATH};
use log::warn;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{Module, Stage};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct StagesExt<'a> {
    #[builder(default, setter(into))]
    pub stages: Cow<'a, [Stage<'a>]>,
}

impl<'a> StagesExt<'a> {
    /// Parse a module file returning a [`StagesExt`]
    ///
    /// # Errors
    /// Can return an `anyhow` Error if the file cannot be read or deserialized
    /// into a [`StagesExt`]
    pub fn parse_stage_from_file(file_name: &str) -> Result<Self> {
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
                let mut stage = serde_yaml::from_str::<Stage>(&file)
                    .map_err(blue_build_utils::serde_yaml_err(&file))?;
                if let Some(ref mut modules_ext) = stage.modules_ext {
                    modules_ext.modules = Module::get_modules(&modules_ext.modules)?.into();
                }
                Ok(Self::builder().stages(vec![stage]).build())
            },
            |mut stages_ext| -> Result<Self> {
                let mut stages: Vec<Stage> =
                    stages_ext.stages.iter().map(ToOwned::to_owned).collect();
                for stage in &mut stages {
                    if let Some(ref mut modules_ext) = stage.modules_ext {
                        modules_ext.modules = Module::get_modules(&modules_ext.modules)?.into();
                    }
                }
                stages_ext.stages = stages.into();
                Ok(stages_ext)
            },
        )
    }
}
