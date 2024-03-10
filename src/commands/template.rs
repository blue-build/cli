use std::path::PathBuf;

use anyhow::Result;
use blue_build_recipe::Recipe;
use blue_build_template::{ContainerFileTemplate, Template};
use blue_build_utils::constants::*;
use clap::Args;
use log::{debug, info, trace};
use typed_builder::TypedBuilder;

use crate::strategies::{self, BUILD_ID};

use super::BlueBuildCommand;

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
                .unwrap_or_else(|| PathBuf::from(RECIPE_PATH))
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
            .unwrap_or_else(|| PathBuf::from(RECIPE_PATH));

        debug!("Deserializing recipe");
        let recipe_de = Recipe::parse(&recipe_path)?;
        trace!("recipe_de: {recipe_de:#?}");

        let template = ContainerFileTemplate::builder()
            .os_version(strategies::get_os_version(&recipe_de)?)
            .build_id(*BUILD_ID)
            .recipe(&recipe_de)
            .recipe_path(recipe_path.as_path())
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
