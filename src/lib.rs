use std::{fs, path::PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use recipe::Recipe;
use tera::{Context, Tera};

pub const DEFAULT_CONTAINERFILE: &'static str =
    include_str!("../templates/starting_point.template");

pub mod recipe;

#[derive(Parser, Debug)]
#[command(name = "Ublue Builder", author, version, about, long_about = None)]
pub struct UblueArgs {
    #[command(subcommand)]
    pub command: CommandArgs,
}

#[derive(Debug, Subcommand)]
pub enum CommandArgs {
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

pub fn setup_tera(recipe: String) -> Result<(Tera, Context)> {
    let recipe_de =
        serde_yaml::from_str::<Recipe>(fs::read_to_string(PathBuf::from(&recipe))?.as_str())?
            .process_repos();

    let mut context = Context::from_serialize(recipe_de)?;
    context.insert("recipe", &recipe);

    let mut tera = Tera::default();
    tera.add_raw_template("Containerfile", DEFAULT_CONTAINERFILE)?;

    Ok((tera, context))
}
