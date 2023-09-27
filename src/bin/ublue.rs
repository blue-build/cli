use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tera::{Context, Tera};
use ublue_rs::{Recipe, DEFAULT_CONTAINERFILE};

#[derive(Parser, Debug)]
#[command(name = "Ublue Builder", author, version, about, long_about = None)]
struct UblueArgs {
    #[command(subcommand)]
    command: CommandArgs,
}

#[derive(Debug, Subcommand)]
enum CommandArgs {
    /// Generate a Containerfile from a recipe
    Template {
        /// The recipe file to create a template from
        #[arg()]
        recipe: String,

        /// Optional Containerfile to use as a template
        #[arg(short, long)]
        containerfile: Option<String>,
    },

    /// Build an image from a Containerfile
    Build {
        #[arg()]
        containerfile: String,
    },
}

fn main() -> Result<()> {
    let args = UblueArgs::parse();

    match args.command {
        CommandArgs::Template {
            recipe,
            containerfile: _,
        } => {
            let mut recipe_de: Recipe =
                serde_yaml::from_str(fs::read_to_string(PathBuf::from(&recipe))?.as_str())?;

            recipe_de.rpm.repos = recipe_de
                .rpm
                .repos
                .iter()
                .map(|s| {
                    s.replace(
                        "%FEDORA_VERSION%",
                        recipe_de.fedora_version.to_string().as_str(),
                    )
                })
                .collect();

            let mut context = Context::from_serialize(recipe_de)?;
            context.insert("recipe", &recipe);

            let mut tera = Tera::default();
            tera.add_raw_template("Containerfile", DEFAULT_CONTAINERFILE)?;
            let output = tera.render("Containerfile", &context)?;
            println!("{output}");
        }
        CommandArgs::Build { containerfile: _ } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
