use std::{
    env,
    path::{Path, PathBuf},
};

use blue_build_recipe::Recipe;
use blue_build_template::{
    ContainerFileTemplate, OstreeContainerFileTemplate, VanillaContainerFileTemplate,
};
use blue_build_utils::{
    constants::{
        CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_REGISTRY, CONFIG_PATH, GITHUB_REPOSITORY_OWNER,
        RECIPE_FILE, RECIPE_PATH,
    },
    syntax_highlighting::{self, DefaultThemes},
};
use clap::{crate_version, Args};
use log::{debug, info, trace, warn};
use miette::{IntoDiagnostic, Result};
use typed_builder::TypedBuilder;

use crate::{drivers::Driver, shadow};

use super::{BlueBuildCommand, DriverArgs};

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct GenerateCommand {
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

    /// Instead of creating a Containerfile, display
    /// the full recipe after traversing all `from-file` properties.
    ///
    /// This can be used to help debug the order
    /// you defined your recipe.
    #[arg(short, long)]
    #[builder(default)]
    display_full_recipe: bool,

    /// Choose a theme for the syntax highlighting
    /// for the Containerfile or Yaml.
    ///
    /// The default is `mocha-dark`.
    #[arg(short = 't', long)]
    #[builder(default, setter(strip_option))]
    syntax_theme: Option<DefaultThemes>,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for GenerateCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::builder()
            .build_driver(self.drivers.build_driver)
            .inspect_driver(self.drivers.inspect_driver)
            .build()
            .init();

        self.template_file()
    }
}

impl GenerateCommand {
    fn template_file(&self) -> Result<()> {
        trace!("TemplateCommand::template_file()");

        let recipe_path = self.recipe.clone().unwrap_or_else(|| {
            let legacy_path = Path::new(CONFIG_PATH);
            let recipe_path = Path::new(RECIPE_PATH);
            if recipe_path.exists() && recipe_path.is_dir() {
                recipe_path.join(RECIPE_FILE)
            } else {
                warn!("Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}");
                legacy_path.join(RECIPE_FILE)
            }
        });

        debug!("Deserializing recipe");
        let recipe_de = Recipe::parse(&recipe_path)?;
        trace!("recipe_de: {recipe_de:#?}");

        if self.display_full_recipe {
            if let Some(output) = self.output.as_ref() {
                std::fs::write(output, serde_yaml::to_string(&recipe_de).into_diagnostic()?)
                    .into_diagnostic()?;
            } else {
                syntax_highlighting::print_ser(&recipe_de, "yml", self.syntax_theme)?;
            }
            return Ok(());
        }

        info!("Templating for recipe at {}", recipe_path.display());

        let template: Box<dyn ContainerFileTemplate> = match &recipe_de.base_image_type {
            Some(cow) => match cow.as_ref() {
                "vanilla" => Box::new(self.build_vanilla_template(&recipe_de, &recipe_path)?),
                "ostree" => Box::new(self.build_ostree_template(&recipe_de, &recipe_path)?),
                _ => Box::new(self.build_ostree_template(&recipe_de, &recipe_path)?),
            },
            None => Box::new(self.build_ostree_template(&recipe_de, &recipe_path)?),
        };

        let output_str = template.render().into_diagnostic()?;
        if let Some(output) = self.output.as_ref() {
            debug!("Templating to file {}", output.display());
            trace!("Containerfile:\n{output_str}");

            std::fs::write(output, output_str).into_diagnostic()?;
        } else {
            debug!("Templating to stdout");
            syntax_highlighting::print(&output_str, "Dockerfile", self.syntax_theme)?;
        }

        Ok(())
    }

    fn build_ostree_template<'a>(
        &self,
        recipe_de: &'a Recipe<'a>,
        recipe_path: &'a Path,
    ) -> Result<OstreeContainerFileTemplate<'a>> {
        info!("Using ostree template");
        Ok(OstreeContainerFileTemplate::builder()
            .os_version(Driver::get_os_version(recipe_de)?)
            .build_id(Driver::get_build_id())
            .recipe(recipe_de)
            .recipe_path(recipe_path)
            .registry(self.get_registry())
            .exports_tag({
                #[allow(clippy::const_is_empty)]
                if shadow::COMMIT_HASH.is_empty() {
                    // This is done for users who install via
                    // cargo. Cargo installs do not carry git
                    // information via shadow
                    format!("v{}", crate_version!())
                } else {
                    shadow::COMMIT_HASH.to_string()
                }
            })
            .build())
    }

    fn build_vanilla_template<'a>(
        &self,
        recipe_de: &'a Recipe<'a>,
        recipe_path: &'a Path,
    ) -> Result<VanillaContainerFileTemplate<'a>> {
        info!("Using vanilla template");
        Ok(VanillaContainerFileTemplate::builder()
            .os_version(Driver::get_os_version(recipe_de)?)
            .build_id(Driver::get_build_id())
            .recipe(recipe_de)
            .recipe_path(recipe_path)
            .registry(self.get_registry())
            .exports_tag({
                #[allow(clippy::const_is_empty)]
                if shadow::COMMIT_HASH.is_empty() {
                    // This is done for users who install via
                    // cargo. Cargo installs do not carry git
                    // information via shadow
                    format!("v{}", crate_version!())
                } else {
                    shadow::COMMIT_HASH.to_string()
                }
            })
            .build())
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
