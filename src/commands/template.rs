use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::Result;
use askama::Template;
use clap::Args;
use log::{debug, error, info, trace};
use typed_builder::TypedBuilder;

use crate::{
    constants::{self},
    module_recipe::{Module, Recipe},
};

use super::BlueBuildCommand;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe<'a>,
    recipe_path: &'a Path,

    #[builder(default)]
    export_script: ExportsTemplate,
}

#[derive(Debug, Clone, Default, Template)]
#[template(path = "export.sh", escape = "none")]
pub struct ExportsTemplate;

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct TemplateCommand {
    /// The recipe file to create a template from
    #[arg()]
    #[builder(default, setter(into, strip_option))]
    recipe: Option<PathBuf>,

    /// File to output to instead of STDOUT
    #[arg(short, long)]
    #[builder(default, setter(into, strip_option))]
    output: Option<PathBuf>,
}

impl BlueBuildCommand for TemplateCommand {
    fn try_run(&mut self) -> Result<()> {
        info!(
            "Templating for recipe at {}",
            self.recipe
                .clone()
                .unwrap_or_else(|| PathBuf::from(constants::RECIPE_PATH))
                .display()
        );

        self.template_file()
    }
}

impl TemplateCommand {
    fn template_file(&self) -> Result<()> {
        trace!("TemplateCommand::template_file()");

        let recipe_path = self
            .recipe
            .clone()
            .unwrap_or_else(|| PathBuf::from(constants::RECIPE_PATH));

        debug!("Deserializing recipe");
        let recipe_de = Recipe::parse(&recipe_path)?;
        trace!("recipe_de: {recipe_de:#?}");

        let template = ContainerFileTemplate::builder()
            .recipe(&recipe_de)
            .recipe_path(&recipe_path)
            .build();

        let output_str = template.render()?;
        if let Some(output) = self.output.as_ref() {
            debug!("Templating to file {}", output.display());
            trace!("Containerfile:\n{output_str}");

            std::fs::write(output, output_str)?;
        } else {
            debug!("Templating to stdout");
            println!("{output_str}");
        }

        info!("Finished templating Containerfile");
        Ok(())
    }
}

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //

fn print_script(script_contents: &ExportsTemplate) -> String {
    trace!("print_script({script_contents})");

    format!(
        "\"{}\"",
        script_contents
            .render()
            .unwrap_or_else(|e| {
                error!("Failed to render export.sh script: {e}");
                process::exit(1);
            })
            .replace('\n', "\\n")
            .replace('\"', "\\\"")
            .replace('$', "\\$")
    )
}

fn has_cosign_file() -> bool {
    trace!("has_cosign_file()");
    std::env::current_dir()
        .map(|p| p.join(constants::COSIGN_PATH).exists())
        .unwrap_or(false)
}

#[must_use]
fn get_module_type_list(module: &Module, typ: &str, list_key: &str) -> Option<Vec<String>> {
    if module.module_type.as_ref()? == typ {
        Some(
            module
                .config
                .get(list_key)?
                .as_sequence()?
                .iter()
                .filter_map(|t| Some(t.as_str()?.to_owned()))
                .collect(),
        )
    } else {
        None
    }
}

#[must_use]
fn get_containerfile_list(module: &Module) -> Option<Vec<String>> {
    get_module_type_list(module, "containerfile", "containerfiles")
}

#[must_use]
fn get_containerfile_snippets(module: &Module) -> Option<Vec<String>> {
    get_module_type_list(module, "containerfile", "snippets")
}

#[must_use]
fn print_containerfile(containerfile: &str) -> String {
    debug!("print_containerfile({containerfile})");
    debug!("Loading containerfile contents for {containerfile}");

    let path = format!("config/containerfiles/{containerfile}/Containerfile");

    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
        error!("Failed to read file {path}: {e}");
        process::exit(1);
    });

    debug!("Containerfile contents {path}:\n{file}");

    file
}

fn print_module_context(module: &Module) -> String {
    serde_json::to_string(module).unwrap_or_else(|e| {
        error!("Failed to parse module!!!!!: {e}");
        process::exit(1);
    })
}

fn get_files_list(module: &Module) -> Option<Vec<(String, String)>> {
    Some(
        module
            .config
            .get("files")?
            .as_sequence()?
            .iter()
            .filter_map(|entry| entry.as_mapping())
            .flatten()
            .filter_map(|(src, dest)| {
                Some((
                    format!("./config/files/{}", src.as_str()?),
                    dest.as_str()?.to_string(),
                ))
            })
            .collect(),
    )
}

fn get_github_repo_owner() -> Option<String> {
    Some(env::var("GITHUB_REPOSITORY_OWNER").ok()?.to_lowercase())
}

fn get_gitlab_registry_path() -> Option<String> {
    Some(
        format!(
            "{}/{}/{}",
            env::var("CI_REGISTRY").ok()?,
            env::var("CI_PROJECT_NAMESPACE").ok()?,
            env::var("CI_PROJECT_NAME").ok()?,
        )
        .to_lowercase(),
    )
}
