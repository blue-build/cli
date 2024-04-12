use std::{env, path::PathBuf};

use anyhow::Result;
use blue_build_recipe::Recipe;
use blue_build_template::{ContainerFileTemplate, Template};
use blue_build_utils::constants::{
    CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_REGISTRY, GITHUB_REPOSITORY_OWNER, RECIPE_PATH,
};
use clap::Args;
use log::{debug, info, trace};
use typed_builder::TypedBuilder;

use crate::{drivers::Driver, shadow};

use super::{BlueBuildCommand, DriverArgs};

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

    /// The registry domain the image will be published to.
    ///
    /// This is used for modules that need to know where
    /// the image is being published (i.e. the signing module).
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    registry: Option<String>,

    /// The registry namespace the image will be published to.
    ///
    /// This is used for modules that need to know where
    /// the image is being published (i.e. the signing module).
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    registry_namespace: Option<String>,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
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

        Driver::builder()
            .build_driver(self.drivers.build_driver)
            .inspect_driver(self.drivers.inspect_driver)
            .build()
            .init()?;

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
            .os_version(Driver::get_os_version(&recipe_de)?)
            .build_id(Driver::get_build_id())
            .recipe(&recipe_de)
            .recipe_path(recipe_path.as_path())
            .registry(self.get_registry())
            .exports_tag(shadow::BB_COMMIT_HASH)
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

    fn get_registry(&self) -> String {
        match (
            self.registry.as_ref(),
            self.registry_namespace.as_ref(),
            Self::get_github_repo_owner(),
            Self::get_gitlab_registry_path(),
        ) {
            (Some(r), Some(rn), _, _) => format!("{r}/{rn}"),
            (Some(r), None, _, _) => r.to_string(),
            (None, None, Some(gh_repo_owner), None) => format!("ghcr.io/{gh_repo_owner}"),
            (None, None, None, Some(gl_reg_path)) => gl_reg_path,
            _ => "localhost".to_string(),
        }
    }

    fn get_github_repo_owner() -> Option<String> {
        Some(env::var(GITHUB_REPOSITORY_OWNER).ok()?.to_lowercase())
    }

    fn get_gitlab_registry_path() -> Option<String> {
        Some(
            format!(
                "{}/{}/{}",
                env::var(CI_REGISTRY).ok()?,
                env::var(CI_PROJECT_NAMESPACE).ok()?,
                env::var(CI_PROJECT_NAME).ok()?,
            )
            .to_lowercase(),
        )
    }
}

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //
