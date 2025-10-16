use std::{
    fmt::Write,
    fs::OpenOptions,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use blue_build_process_management::ASYNC_RUNTIME;
use blue_build_recipe::{FromFileList, ModuleExt, Recipe, StagesExt};
use blue_build_utils::constants::{
    MODULE_STAGE_LIST_V1_SCHEMA_URL, MODULE_V1_SCHEMA_URL, RECIPE_V1_SCHEMA_URL,
    STAGE_V1_SCHEMA_URL,
};
use bon::Builder;
use clap::Args;
use colored::Colorize;
use log::{debug, info, trace};
use miette::{Context, IntoDiagnostic, Report, bail, miette};
use rayon::prelude::*;
use schema_validator::SchemaValidator;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::BlueBuildCommand;

mod location;
mod schema_validator;
mod yaml_span;

#[derive(Debug, Args, Builder)]
pub struct ValidateCommand {
    /// The path to the recipe.
    ///
    /// NOTE: In order for this to work,
    /// you must be in the root of your
    /// bluebuild repository.
    pub recipe: PathBuf,

    /// Display all errors that failed
    /// validation of the recipe.
    #[arg(short, long)]
    #[builder(default)]
    pub all_errors: bool,

    #[clap(skip)]
    recipe_validator: Option<SchemaValidator>,

    #[clap(skip)]
    stage_validator: Option<SchemaValidator>,

    #[clap(skip)]
    module_validator: Option<SchemaValidator>,

    #[clap(skip)]
    module_stage_list_validator: Option<SchemaValidator>,
}

impl BlueBuildCommand for ValidateCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        let recipe_path_display = self.recipe.display().to_string().bold().italic();

        if !self.recipe.is_file() {
            bail!("File {recipe_path_display} must exist");
        }

        ASYNC_RUNTIME
            .block_on(self.setup_validators())
            .wrap_err("Failed to setup validators")?;

        if let Err(errors) = self.validate_recipe() {
            let errors = errors.into_iter().try_fold(
                String::new(),
                |mut full, err| -> miette::Result<String> {
                    write!(&mut full, "{err:?}").into_diagnostic()?;
                    Ok(full)
                },
            )?;
            let main_err = format!("Recipe {recipe_path_display} failed to validate");

            if self.all_errors {
                return Err(miette!("{errors}").context(main_err));
            }

            return Err(miette!(
                help = format!(
                    "Use `{}` to view more information.\n{}",
                    format!("bluebuild validate --all-errors {}", self.recipe.display()).bold(),
                    format_args!(
                        "If you're using a local module, be sure to add `{}` to the module entry",
                        "source: local".bold()
                    ),
                ),
                "{errors}",
            )
            .context(main_err));
        }
        info!("Recipe {recipe_path_display} is valid");

        Ok(())
    }
}

impl ValidateCommand {
    async fn setup_validators(&mut self) -> Result<(), Report> {
        let (rv, sv, mv, mslv) = tokio::try_join!(
            SchemaValidator::builder()
                .url(RECIPE_V1_SCHEMA_URL)
                .all_errors(self.all_errors)
                .build(),
            SchemaValidator::builder()
                .url(STAGE_V1_SCHEMA_URL)
                .all_errors(self.all_errors)
                .build(),
            SchemaValidator::builder()
                .url(MODULE_V1_SCHEMA_URL)
                .all_errors(self.all_errors)
                .build(),
            SchemaValidator::builder()
                .url(MODULE_STAGE_LIST_V1_SCHEMA_URL)
                .all_errors(self.all_errors)
                .build(),
        )?;
        self.recipe_validator = Some(rv);
        self.stage_validator = Some(sv);
        self.module_validator = Some(mv);
        self.module_stage_list_validator = Some(mslv);
        Ok(())
    }

    fn validate_file<DF>(
        &self,
        path: &Path,
        traversed_files: &[&Path],
        single_validator: &SchemaValidator,
    ) -> Vec<Report>
    where
        DF: DeserializeOwned + FromFileList,
    {
        let path_display = path.display().to_string().bold().italic();

        if traversed_files.contains(&path) {
            return vec![miette!(
                "{} File {path_display} has already been parsed:\n{traversed_files:?}",
                "Circular dependency detected!".bright_red(),
            )];
        }
        let traversed_files = {
            let mut files: Vec<&Path> = Vec::with_capacity(traversed_files.len() + 1);
            files.extend_from_slice(traversed_files);
            files.push(path);
            files
        };

        let file_str = match read_file(path) {
            Err(e) => return vec![e],
            Ok(f) => Arc::new(f),
        };

        match serde_yaml::from_str::<Value>(&file_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize file {path_display}"))
        {
            Ok(instance) => {
                trace!("{path_display}:\n{instance}");

                if instance.get(DF::LIST_KEY).is_some() {
                    debug!("{path_display} is a list file");
                    let err = self
                        .module_stage_list_validator
                        .as_ref()
                        .unwrap()
                        .process_validation(path, file_str.clone())
                        .err();

                    err.map_or_else(
                        || {
                            serde_yaml::from_str::<DF>(&file_str)
                                .into_diagnostic()
                                .map_or_else(
                                    |e| vec![e],
                                    |file| {
                                        let mut errs = file
                                            .get_from_file_paths()
                                            .par_iter()
                                            .map(|file_path| {
                                                self.validate_file::<DF>(
                                                    file_path,
                                                    &traversed_files,
                                                    single_validator,
                                                )
                                            })
                                            .flatten()
                                            .collect::<Vec<_>>();
                                        errs.extend(
                                            file.get_module_from_file_paths()
                                                .par_iter()
                                                .map(|file_path| {
                                                    self.validate_file::<ModuleExt>(
                                                        file_path,
                                                        &[],
                                                        self.module_validator.as_ref().unwrap(),
                                                    )
                                                })
                                                .flatten()
                                                .collect::<Vec<_>>(),
                                        );
                                        errs
                                    },
                                )
                        },
                        |err| vec![err.into()],
                    )
                } else {
                    debug!("{path_display} is a single file file");
                    single_validator
                        .process_validation(path, file_str)
                        .map_or_else(|e| vec![e.into()], |()| Vec::new())
                }
            }
            Err(e) => vec![e],
        }
    }

    fn validate_recipe(&self) -> Result<(), Vec<Report>> {
        let recipe_path_display = self.recipe.display().to_string().bold().italic();
        debug!("Validating recipe {recipe_path_display}");

        let recipe_str = Arc::new(read_file(&self.recipe).map_err(err_vec)?);
        let recipe: Value = serde_yaml::from_str(&recipe_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize recipe {recipe_path_display}"))
            .map_err(err_vec)?;
        trace!("{recipe_path_display}:\n{recipe}");

        let schema_validator = self.recipe_validator.as_ref().unwrap();
        let err = schema_validator
            .process_validation(&self.recipe, recipe_str.clone())
            .err();

        if let Some(err) = err {
            Err(vec![err.into()])
        } else {
            let recipe: Recipe = serde_yaml::from_str(&recipe_str)
                .into_diagnostic()
                .with_context(|| {
                    format!("Unable to convert Value to Recipe for {recipe_path_display}")
                })
                .map_err(err_vec)?;

            let mut errors: Vec<Report> = Vec::new();
            if let Some(stages) = &recipe.stages_ext {
                debug!("Validating stages for recipe {recipe_path_display}");

                errors.extend(
                    stages
                        .get_from_file_paths()
                        .par_iter()
                        .map(|stage_path| {
                            debug!(
                                "Found 'from-file' reference in {recipe_path_display} going to {}",
                                stage_path.display().to_string().italic().bold()
                            );
                            self.validate_file::<StagesExt>(
                                stage_path,
                                &[],
                                self.stage_validator.as_ref().unwrap(),
                            )
                        })
                        .flatten()
                        .collect::<Vec<_>>(),
                );
            }

            debug!("Validating modules for recipe {recipe_path_display}");
            errors.extend(
                recipe
                    .modules_ext
                    .get_from_file_paths()
                    .par_iter()
                    .map(|module_path| {
                        debug!(
                            "Found 'from-file' reference in {recipe_path_display} going to {}",
                            module_path.display().to_string().italic().bold()
                        );
                        self.validate_file::<ModuleExt>(
                            module_path,
                            &[],
                            self.module_validator.as_ref().unwrap(),
                        )
                    })
                    .flatten()
                    .collect::<Vec<_>>(),
            );
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }
}

fn err_vec(err: Report) -> Vec<Report> {
    vec![err]
}

fn read_file(path: &Path) -> Result<String, Report> {
    let mut recipe = String::new();
    BufReader::new(
        OpenOptions::new()
            .read(true)
            .open(path)
            .into_diagnostic()
            .with_context(|| {
                format!(
                    "Unable to open {}",
                    path.display().to_string().italic().bold()
                )
            })?,
    )
    .read_to_string(&mut recipe)
    .into_diagnostic()?;
    Ok(recipe)
}
