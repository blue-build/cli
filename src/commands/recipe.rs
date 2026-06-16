use std::{fs::OpenOptions, io::Write, path::PathBuf};

use blue_build_recipe::Recipe;
use bon::Builder;
use clap::{Args, Subcommand};
use log::debug;
use miette::{Context, IntoDiagnostic};

use crate::commands::BlueBuildCommand;

#[derive(Debug, Args, Builder)]
pub struct RecipeCommand {
    /// The operation to perform on recipes
    #[command(subcommand)]
    subcommand: RecipeSubCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum RecipeSubCommand {
    /// Upgrade your recipe files to the latest schema version
    Upgrade {
        /// The paths to the recipe files
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
}

impl BlueBuildCommand for RecipeCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        match &self.subcommand {
            RecipeSubCommand::Upgrade { paths } => {
                for path in paths {
                    debug!("Opening file {} for reading", path.display());
                    let file = OpenOptions::new()
                        .read(true)
                        .open(path)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("Failed to open {} for reading", path.display())
                        })?;

                    debug!("Deserializing file {}", path.display());
                    let recipe: Recipe = serde_yaml::from_reader(&file)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("Failed to deserialize recipe file {}", path.display())
                        })?;
                    drop(file);

                    debug!("Upgrading recipe");
                    let recipe = recipe.upgrade();

                    debug!("Opening file {} for writing", path.display());
                    let file = &mut OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(path)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("Failed to open {} for writing", path.display())
                        })?;

                    debug!("Writing schema header to file {}", path.display());
                    writeln!(
                        file,
                        concat!(
                            "---\n",
                            "# yaml-language-server: ",
                            "$schema=https://schema.blue-build.org/recipe.json",
                        ),
                    )
                    .into_diagnostic()
                    .wrap_err_with(|| format!("Failed to write recipe file {}", path.display()))?;

                    debug!("Writing recipe to file {}", path.display());
                    serde_yaml::to_writer(file, &recipe)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("Failed to write recipe file {}", path.display())
                        })?;
                }
            }
        }
        Ok(())
    }
}
