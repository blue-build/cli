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

use crate::module_recipe::{Module, ModuleExt, Recipe};

use super::BlueBuildCommand;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe,
    recipe_path: &'a Path,

    module_template: ModuleTemplate<'a>,

    #[builder(default)]
    export_script: ExportsTemplate,
}

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile.module", escape = "none")]
pub struct ModuleTemplate<'a> {
    module_ext: &'a ModuleExt,
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
        let recipe_de = Recipe::parse(&self.recipe)?;
        trace!("recipe_de: {recipe_de:#?}");

        let template = ContainerFileTemplate::builder()
            .recipe(&recipe_de)
            .recipe_path(&self.recipe)
            .module_template(
                ModuleTemplate::builder()
                    .module_ext(&recipe_de.modules_ext)
                    .build(),
            )
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

#[must_use]
pub fn get_containerfile_list(module: &Module) -> Option<Vec<String>> {
    if module.module_type.as_ref()? == "containerfile" {
        Some(
            module
                .config
                .get("containerfiles")?
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
pub fn print_containerfile(containerfile: &str) -> String {
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

#[must_use]
pub fn template_module_from_file(file_name: &str) -> String {
    debug!("get_module_from_file({file_name})");

    let file_path = PathBuf::from("config").join(file_name);
    let file = fs::read_to_string(file_path).unwrap_or_else(|e| {
        error!("Failed to read module {file_name}: {e}");
        String::default()
    });

    let template_err_fn = |e| {
        error!("Failed to render module {file_name}: {e}");
        process::exit(1);
    };

    serde_yaml::from_str::<ModuleExt>(file.as_str()).map_or_else(
        |_| {
            let module = serde_yaml::from_str::<Module>(file.as_str()).unwrap_or_else(|e| {
                error!("Failed to deserialize module {file_name}: {e}");
                process::exit(1);
            });

            ModuleTemplate::builder()
                .module_ext(&ModuleExt::builder().modules(vec![module]).build())
                .build()
                .render()
                .unwrap_or_else(template_err_fn)
        },
        |module_ext| {
            ModuleTemplate::builder()
                .module_ext(&module_ext)
                .build()
                .render()
                .unwrap_or_else(template_err_fn)
        },
    )
}

fn print_module_context(module: &Module) -> String {
    serde_json::to_string(module).unwrap_or_else(|e| {
        error!("Failed to parse module: {e}");
        process::exit(1);
    })
}
