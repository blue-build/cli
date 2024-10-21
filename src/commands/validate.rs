use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use blue_build_recipe::{ModuleExt, Recipe, StagesExt};
use blue_build_utils::{
    string,
    syntax_highlighting::{self, DefaultThemes},
    traits::AsRefCollector,
};
use bon::Builder;
use cached::proc_macro::cached;
use clap::Args;
use colored::Colorize;
use jsonschema::{paths::Location, Retrieve, Uri, ValidationError, Validator};
use log::{debug, info, trace};
use miette::{bail, miette, Context, IntoDiagnostic, Report};
use rayon::prelude::*;
use serde_json::Value;

use super::BlueBuildCommand;

const BASE_SCHEMA_URL: &str = "https://schema.blue-build.org";
const RECIPE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/recipe-v1.json";
const STAGE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/stage-v1.json";
const STAGE_LIST_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/stage-list-v1.json";
const MODULE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/module-v1.json";
const MODULE_LIST_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/module-list-v1.json";

#[derive(Debug, Args, Builder)]
pub struct ValidateCommand {
    pub recipe: PathBuf,

    /// Choose a theme for the syntax highlighting
    /// for the Containerfile or Yaml.
    ///
    /// The default is `mocha-dark`.
    #[arg(short = 't', long)]
    syntax_theme: Option<DefaultThemes>,

    #[clap(skip)]
    recipe_validator: Option<Validator>,

    #[clap(skip)]
    stage_validator: Option<Validator>,

    #[clap(skip)]
    stage_list_validator: Option<Validator>,

    #[clap(skip)]
    module_validator: Option<Validator>,

    #[clap(skip)]
    module_list_validator: Option<Validator>,
}

impl BlueBuildCommand for ValidateCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        let recipe_path_display = self.recipe.display().to_string().bold().italic();

        if !self.recipe.is_file() {
            bail!("File {recipe_path_display} must exist");
        }

        self.setup_validators()?;

        if let Err(errors) = self.validate_recipe() {
            bail!(
                "Recipe {recipe_path_display} failed to validate:\n{}",
                errors.into_iter().fold(String::new(), |mut full, err| {
                    full.push_str(&format!("{err:?}"));
                    full
                })
            );
        }
        info!("Recipe {recipe_path_display} is valid");

        Ok(())
    }
}

impl ValidateCommand {
    fn setup_validators(&mut self) -> Result<(), Report> {
        self.recipe_validator = Some(build_validator(RECIPE_V1_SCHEMA_URL)?);
        self.stage_validator = Some(build_validator(STAGE_V1_SCHEMA_URL)?);
        self.stage_list_validator = Some(build_validator(STAGE_LIST_V1_SCHEMA_URL)?);
        self.module_validator = Some(build_validator(MODULE_V1_SCHEMA_URL)?);
        self.module_list_validator = Some(build_validator(MODULE_LIST_V1_SCHEMA_URL)?);
        Ok(())
    }

    fn validate_stage_file<'a>(
        &self,
        path: &'a Path,
        mut traversed_files: Vec<&'a Path>,
    ) -> Result<(), Vec<Report>> {
        let path_display = path.display().to_string().bold().italic();

        if traversed_files.contains(&path) {
            return Err(vec![miette!(
                "{} File {path_display} has already been parsed:\n{traversed_files:?}",
                "Circular dependency detected!".bright_red(),
            )]);
        }
        traversed_files.push(path);

        let stage_str = read_file(path).map_err(err_vec)?;
        let stage: Value = serde_yaml::from_str(&stage_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize stage {path_display}"))
            .map_err(err_vec)?;
        trace!("{path_display}:\n{stage}");

        let stage_valid = self
            .stage_validator
            .as_ref()
            .unwrap()
            .validate(&stage)
            .map_err(Iterator::collect::<Vec<_>>);

        if let Err(mut e1) = stage_valid {
            debug!("{path_display} failed to validate as single stage, checking stage list");
            self.stage_list_validator
                .as_ref()
                .unwrap()
                .validate(&stage)
                .map_err(|e2| {
                    e1.extend(e2);
                    e1.into_iter()
                })
                .map_err(validate_err(path, self.syntax_theme))?;

            debug!("{path_display} is a multi stage file");

            let stages: StagesExt = serde_yaml::from_str(&stage_str)
                .into_diagnostic()
                .map_err(err_vec)?;

            let errors = stages
                .get_from_file_paths()
                .par_iter()
                .map(|stage_path| {
                    debug!(
                        "Found 'from-file' reference in {path_display} going to {}",
                        stage_path.display().to_string().italic().bold()
                    );

                    self.validate_stage_file(stage_path, traversed_files.collect_as_ref_vec())
                })
                .filter_map(Result::err)
                .flatten()
                .collect::<Vec<_>>();

            if !errors.is_empty() {
                return Err(errors);
            }
        }

        Ok(())
    }

    fn validate_module_file<'a>(
        &self,
        path: &'a Path,
        mut traversed_files: Vec<&'a Path>,
    ) -> Result<(), Vec<Report>> {
        let path_display = path.display().to_string().bold().italic();
        debug!("Validating module file {path_display}");

        if traversed_files.contains(&path) {
            return Err(vec![miette!(
                "{} File {path_display} has already been parsed:\n{traversed_files:?}",
                "Circular dependency detected!".bright_red(),
            )]);
        }
        traversed_files.push(path);

        let module_str = read_file(path).map_err(err_vec)?;
        let module: Value = serde_yaml::from_str(&module_str)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize module {path_display}"))
            .map_err(err_vec)?;
        trace!("{path_display}:\n{module}");

        let module_validated = self
            .module_validator
            .as_ref()
            .unwrap()
            .validate(&module)
            .map_err(Iterator::collect::<Vec<_>>);

        if let Err(mut e1) = module_validated {
            debug!("{path_display} failed to validate as single module, checking module list");
            self.module_list_validator
                .as_ref()
                .unwrap()
                .validate(&module)
                .map_err(|e2| {
                    e1.extend(e2);
                    e1.into_iter()
                })
                .map_err(validate_err(path, self.syntax_theme))?;

            debug!("{path_display} is a multi module file");

            let modules: ModuleExt = serde_yaml::from_str(&module_str)
                .into_diagnostic()
                .map_err(err_vec)?;

            let errors = modules
                .get_from_file_paths()
                .par_iter()
                .map(|module_path| {
                    debug!(
                        "Found 'from-file' reference in {path_display} going to {}",
                        module_path.display().to_string().italic().bold()
                    );

                    self.validate_module_file(module_path, traversed_files.collect_as_ref_vec())
                })
                .filter_map(Result::err)
                .flatten()
                .collect::<Vec<_>>();

            if !errors.is_empty() {
                return Err(errors);
            }
        }

        Ok(())
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

        self.recipe_validator
            .as_ref()
            .unwrap()
            .validate(&recipe)
            .map_err(validate_err(&self.recipe, self.syntax_theme))?;
        let recipe: Recipe = serde_yaml::from_str(&recipe_str)
            .into_diagnostic()
            .with_context(|| format!("Unable to convert Value to Recipe for {recipe_path_display}"))
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
                        self.validate_stage_file(stage_path, vec![])
                    })
                    .filter_map(Result::err)
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
                    self.validate_module_file(module_path, vec![])
                })
                .filter_map(Result::err)
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

fn err_vec(err: Report) -> Vec<Report> {
    vec![err]
}

fn build_validator(url: &str) -> Result<Validator, Report> {
    let recipe_schema = reqwest::blocking::get(url)
        .into_diagnostic()
        .with_context(|| format!("Failed to get schema at {url}"))?
        .json()
        .into_diagnostic()
        .with_context(|| format!("Failed to get json for schema {url}"))?;
    jsonschema::options()
        .with_retriever(ModuleSchemaRetriever)
        .build(&recipe_schema)
        .into_diagnostic()
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

fn validate_err<'a, 'b, I>(
    path: &'b Path,
    theme: Option<DefaultThemes>,
) -> impl Fn(I) -> Vec<Report> + use<'a, 'b, I>
where
    I: Iterator<Item = ValidationError<'a>>,
{
    let extract_err = |err: ValidationError<'a>| -> (Cow<'a, Value>, Location) {
        let instance = err.instance;
        let dot_path = err.instance_path;

        (instance, dot_path)
    };

    let process_err = move |(instance, dot_path): (Cow<'_, Value>, Location)| {
        miette!(
            "- Invalid value {} file '{}':\n{}",
            if dot_path.as_str().is_empty() {
                string!("in root of")
            } else {
                format!("at path '{}' in", dot_path.as_str().bold().bright_yellow())
            },
            path.display().to_string().italic().bold(),
            &serde_yaml::to_string(&*instance)
                .into_diagnostic()
                .and_then(|file| syntax_highlighting::highlight(&file, "yml", theme))
                .unwrap_or_else(|_| instance.to_string())
        )
    };

    move |errors: I| {
        let errors = errors
            .inspect(|e| trace!("{}: {e:?}", path.display().to_string().italic().bold()))
            .map(extract_err)
            .collect::<Vec<_>>();
        // .collect::<HashSet<_>>(); // Remove duplicates

        if errors.len() == 1 {
            vec![process_err(errors.into_iter().next().unwrap())]
        } else {
            errors
                .into_iter()
                // .filter(|(_, dot_path)| !dot_path.as_str().is_empty())
                .map(process_err)
                .collect()
        }
    }
}

struct ModuleSchemaRetriever;

impl Retrieve for ModuleSchemaRetriever {
    fn retrieve(
        &self,
        uri: &Uri<&str>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(cache_retrieve(uri)?)
    }
}

#[cached(
    result = true,
    key = "String",
    sync_writes = true,
    convert = r#"{ format!("{uri}") }"#
)]
fn cache_retrieve(uri: &Uri<&str>) -> miette::Result<Value> {
    let scheme = uri.scheme();
    let path = uri.path();

    let uri = match scheme.as_str() {
        "json-schema" => {
            format!("{BASE_SCHEMA_URL}{path}")
        }
        "https" => uri.to_string(),
        scheme => bail!("Unknown scheme {scheme}"),
    };

    debug!("Retrieving schema from {}", uri.bold().italic());
    reqwest::blocking::get(&uri)
        .into_diagnostic()
        .with_context(|| format!("Failed to retrieve schema from {uri}"))?
        .json()
        .into_diagnostic()
        .with_context(|| format!("Failed to parse json from {uri}"))
        .inspect(|value| trace!("{}:\n{value}", uri.bold().italic()))
}
