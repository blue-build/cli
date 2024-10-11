use std::{
    fs::OpenOptions,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use blue_build_process_management::ASYNC_RUNTIME;
use blue_build_recipe::{FromFileList, ModuleExt, Recipe, StagesExt};
use blue_build_utils::{
    string,
    syntax_highlighting::{self},
};
use bon::Builder;
use clap::Args;
use colored::Colorize;
use indexmap::IndexMap;
use jsonschema::{BasicOutput, ValidationError};
use log::{debug, info, trace};
use miette::{bail, miette, Context, IntoDiagnostic, Report};
use rayon::prelude::*;
use schema_validator::{
    build_validator, SchemaValidator, MODULE_LIST_V1_SCHEMA_URL, MODULE_V1_SCHEMA_URL,
    RECIPE_V1_SCHEMA_URL, STAGE_LIST_V1_SCHEMA_URL, STAGE_V1_SCHEMA_URL,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::BlueBuildCommand;

mod schema_validator;

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
    stage_list_validator: Option<SchemaValidator>,

    #[clap(skip)]
    module_validator: Option<SchemaValidator>,

    #[clap(skip)]
    module_list_validator: Option<SchemaValidator>,
}

impl BlueBuildCommand for ValidateCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        let recipe_path_display = self.recipe.display().to_string().bold().italic();

        if !self.recipe.is_file() {
            bail!("File {recipe_path_display} must exist");
        }

        ASYNC_RUNTIME.block_on(self.setup_validators())?;

        if let Err(errors) = self.validate_recipe() {
            let errors = errors.into_iter().fold(String::new(), |mut full, err| {
                full.push_str(&format!("{err:?}"));
                full
            });

            if self.all_errors {
                bail!("Recipe {recipe_path_display} failed to validate:\n{errors}");
            } else {
                bail!(
                    help = format!(
                        "Use `{}` to view more information",
                        format!("bluebuild validate --all-errors {}", self.recipe.display()).bold(),
                    ),
                    "Recipe {recipe_path_display} failed to validate:\n{errors}",
                );
            }
        }
        info!("Recipe {recipe_path_display} is valid");

        Ok(())
    }
}

impl ValidateCommand {
    async fn setup_validators(&mut self) -> Result<(), Report> {
        let (rv, sv, slv, mv, mlv) = tokio::try_join!(
            build_validator(RECIPE_V1_SCHEMA_URL),
            build_validator(STAGE_V1_SCHEMA_URL),
            build_validator(STAGE_LIST_V1_SCHEMA_URL),
            build_validator(MODULE_V1_SCHEMA_URL),
            build_validator(MODULE_LIST_V1_SCHEMA_URL),
        )?;
        self.recipe_validator = Some(rv);
        self.stage_validator = Some(sv);
        self.stage_list_validator = Some(slv);
        self.module_validator = Some(mv);
        self.module_list_validator = Some(mlv);
        Ok(())
    }

    fn validate_file<DF>(
        &self,
        path: &Path,
        traversed_files: &[&Path],
        single_validator: &SchemaValidator,
        list_validator: &SchemaValidator,
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
            Ok(f) => f,
        };

        match serde_yaml::from_str::<Value>(&file_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize file {path_display}"))
        {
            Ok(instance) => {
                trace!("{path_display}:\n{instance}");

                if instance.get(DF::LIST_KEY).is_some() {
                    debug!("{path_display} is a multi file file");
                    let errors = if self.all_errors {
                        process_basic_output(
                            list_validator.validator().apply(&instance).basic(),
                            &instance,
                            path,
                        )
                    } else {
                        list_validator
                            .validator()
                            .iter_errors(&instance)
                            .map(process_err(&self.recipe))
                            .collect()
                    };

                    if errors.is_empty() {
                        match serde_yaml::from_str::<DF>(&file_str).into_diagnostic() {
                            Err(e) => vec![e],
                            Ok(file) => file
                                .get_from_file_paths()
                                .par_iter()
                                .map(|file_path| {
                                    self.validate_file::<DF>(
                                        file_path,
                                        &traversed_files,
                                        single_validator,
                                        list_validator,
                                    )
                                })
                                .flatten()
                                .collect(),
                        }
                    } else {
                        errors
                    }
                } else {
                    debug!("{path_display} is a single file file");
                    if self.all_errors {
                        process_basic_output(
                            single_validator.validator().apply(&instance).basic(),
                            &instance,
                            path,
                        )
                    } else {
                        single_validator
                            .validator()
                            .iter_errors(&instance)
                            .map(|err| miette!("{err}"))
                            .collect()
                    }
                }
            }
            Err(e) => vec![e],
        }
    }

    fn validate_recipe(&self) -> Result<(), Vec<Report>> {
        let recipe_path_display = self.recipe.display().to_string().bold().italic();
        debug!("Validating recipe {recipe_path_display}");

        let recipe_str = read_file(&self.recipe).map_err(err_vec)?;
        let recipe: Value = serde_yaml::from_str(&recipe_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize recipe {recipe_path_display}"))
            .map_err(err_vec)?;
        trace!("{recipe_path_display}:\n{recipe}");

        let schema_validator = self.recipe_validator.as_ref().unwrap();
        let errors = if self.all_errors {
            process_basic_output(
                schema_validator.validator().apply(&recipe).basic(),
                &recipe,
                &self.recipe,
            )
        } else {
            schema_validator
                .validator()
                .iter_errors(&recipe)
                .map(process_err(&self.recipe))
                .collect()
        };

        if errors.is_empty() {
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
                                self.stage_list_validator.as_ref().unwrap(),
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
                            self.module_list_validator.as_ref().unwrap(),
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
        } else {
            Err(errors)
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

fn process_basic_output(out: BasicOutput<'_>, instance: &Value, path: &Path) -> Vec<Report> {
    match out {
        BasicOutput::Valid(_) => vec![],
        BasicOutput::Invalid(errors) => {
            let mut collection: IndexMap<String, Vec<String>> = IndexMap::new();
            let errors = {
                let mut e = errors.into_iter().collect::<Vec<_>>();
                e.sort_by(|e1, e2| {
                    e1.instance_location()
                        .as_str()
                        .cmp(e2.instance_location().as_str())
                });
                e
            };

            for err in errors {
                let schema_path = err.keyword_location();
                let instance_path = err.instance_location().to_string();
                let build_err = || {
                    format!(
                        "{:?}",
                        miette!(
                            "schema_path:'{}'",
                            schema_path.to_string().italic().dimmed(),
                        )
                        .context(err.error_description().to_string().bold().bright_red())
                    )
                };

                collection
                    .entry(instance_path)
                    .and_modify(|errs| {
                        errs.push(build_err());
                        // errs.sort_by(|(path1, _), (path2, _)| path1.cmp(path2));
                    })
                    .or_insert_with(|| vec![build_err()]);
            }

            collection
                .into_iter()
                .map(|(key, value)| {
                    let instance = instance.pointer(&key).unwrap();

                    miette!(
                        "In file {} at '{}':\n\n{}\n{}",
                        path.display().to_string().bold().italic(),
                        key.bold().bright_yellow(),
                        serde_yaml::to_string(instance)
                            .into_diagnostic()
                            .and_then(|file| syntax_highlighting::highlight(&file, "yml", None))
                            .unwrap_or_else(|_| instance.to_string()),
                        value.into_iter().collect::<String>()
                    )
                })
                .collect()
        }
    }
}

fn process_err<'a, 'b>(path: &'b Path) -> impl Fn(ValidationError<'a>) -> Report + use<'a, 'b> {
    move |ValidationError {
              instance,
              instance_path,
              kind: _,
              schema_path: _,
          }| {
        miette!(
            "- Invalid value {} file '{}':\n{}",
            if instance_path.as_str().is_empty() {
                string!("in root of")
            } else {
                format!(
                    "at path '{}' in",
                    instance_path.as_str().bold().bright_yellow()
                )
            },
            path.display().to_string().italic().bold(),
            &serde_yaml::to_string(&*instance)
                .into_diagnostic()
                .and_then(|file| syntax_highlighting::highlight(&file, "yml", None))
                .unwrap_or_else(|_| instance.to_string())
        )
    }
}
