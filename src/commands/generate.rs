use std::{
    env,
    path::{Path, PathBuf},
};

use blue_build_process_management::drivers::{types::Platform, CiDriver, Driver, DriverArgs};
use blue_build_recipe::Recipe;
use blue_build_template::{ContainerFileTemplate, Template};
use blue_build_utils::{
    constants::{CONFIG_PATH, RECIPE_FILE, RECIPE_PATH},
    syntax_highlighting::{self, DefaultThemes},
};
use bon::Builder;
use clap::{crate_version, Args};
use log::{debug, info, trace, warn};
use miette::{IntoDiagnostic, Result};

use crate::shadow;

use super::BlueBuildCommand;

#[derive(Debug, Clone, Args, Builder)]
pub struct GenerateCommand {
    /// The recipe file to create a template from
    #[arg()]
    #[builder(into)]
    recipe: Option<PathBuf>,

    /// File to output to instead of STDOUT
    #[arg(short, long)]
    #[builder(into)]
    output: Option<PathBuf>,

    /// The registry domain the image will be published to.
    ///
    /// This is used for modules that need to know where
    /// the image is being published (i.e. the signing module).
    #[arg(long)]
    #[builder(into)]
    registry: Option<String>,

    /// The registry namespace the image will be published to.
    ///
    /// This is used for modules that need to know where
    /// the image is being published (i.e. the signing module).
    #[arg(long)]
    #[builder(into)]
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
    syntax_theme: Option<DefaultThemes>,

    /// Inspect the image for a specific platform
    /// when retrieving the version.
    #[arg(long, default_value = "native")]
    #[builder(default)]
    platform: Platform,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for GenerateCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::init(self.drivers);

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
        let registry = if let (Some(registry), Some(registry_namespace)) =
            (&self.registry, &self.registry_namespace)
        {
            format!("{registry}/{registry_namespace}")
        } else {
            Driver::get_registry()?
        };

        debug!("Deserializing recipe");
        let recipe = Recipe::parse(&recipe_path)?;
        trace!("recipe_de: {recipe:#?}");

        if self.display_full_recipe {
            if let Some(output) = self.output.as_ref() {
                std::fs::write(output, serde_yaml::to_string(&recipe).into_diagnostic()?)
                    .into_diagnostic()?;
            } else {
                syntax_highlighting::print_ser(&recipe, "yml", self.syntax_theme)?;
            }
            return Ok(());
        }

        info!("Templating for recipe at {}", recipe_path.display());

        let template = ContainerFileTemplate::builder()
            .os_version(
                Driver::get_os_version()
                    .oci_ref(&recipe.base_image_ref()?)
                    .platform(self.platform)
                    .call()?,
            )
            .build_id(Driver::get_build_id())
            .recipe(&recipe)
            .recipe_path(recipe_path.as_path())
            .registry(registry)
            .repo(Driver::get_repo_url()?)
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
            .build();

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
}
