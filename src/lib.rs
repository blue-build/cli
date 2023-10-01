use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use recipe::Recipe;
use tera::{from_value, Context, Function, Tera};

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

fn print_containerfile() -> impl Function {
    Box::new(
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            match args.get("containerfile") {
                Some(v) => match from_value::<String>(v.clone()) {
                    Ok(containerfile) => {
                        Ok(read_to_string(format!("containerfiles/{containerfile}"))?.into())
                    }
                    Err(_) => Err("Arg containerfile wasn't a string".into()),
                },
                None => Err("Needs the argument 'containerfile'".into()),
            }
        },
    )
}

pub fn setup_tera(recipe: String) -> Result<(Tera, Context)> {
    let recipe_de =
        serde_yaml::from_str::<Recipe>(fs::read_to_string(PathBuf::from(&recipe))?.as_str())?
            .process_repos();

    let mut context = Context::from_serialize(recipe_de)?;
    context.insert("recipe", &recipe);

    let mut tera = Tera::default();
    tera.add_raw_template("Containerfile", DEFAULT_CONTAINERFILE)?;
    tera.register_function("print_containerfile", print_containerfile());

    Ok((tera, context))
}
