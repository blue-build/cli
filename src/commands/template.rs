use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Error, Result};
use askama::Template;
use chrono::Local;
use clap::Args;
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::module_recipe::Recipe;

use super::BlueBuildCommand;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe,
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
    #[builder(setter(into))]
    recipe: PathBuf,

    /// File to output to instead of STDOUT
    #[arg(short, long)]
    #[builder(default, setter(into, strip_option))]
    output: Option<PathBuf>,
}

impl BlueBuildCommand for TemplateCommand {
    fn try_run(&mut self) -> Result<()> {
        info!("Templating for recipe at {}", self.recipe.display());

        self.template_file()
    }
}

impl TemplateCommand {
    fn template_file(&self) -> Result<()> {
        trace!("TemplateCommand::template_file()");

        debug!("Deserializing recipe");
        let recipe_de = serde_yaml::from_str::<Recipe>(fs::read_to_string(&self.recipe)?.as_str())?;
        trace!("recipe_de: {recipe_de:#?}");

        let template = ContainerFileTemplate::builder()
            .recipe(&recipe_de)
            .recipe_path(&self.recipe)
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

fn running_gitlab_actions() -> bool {
    trace!(" running_gitlab_actions()");

    env::var("GITHUB_ACTIONS").is_ok_and(|e| e == "true")
}
