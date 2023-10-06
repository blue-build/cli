use std::{
    collections::HashMap,
    fs::{self, read_dir, read_to_string},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use recipe::Recipe;
use tera::{from_value, Context, Tera};

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
        containerfile: Option<PathBuf>,

        /// File to output to instead of STDOUT
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build an image from a Containerfile
    Build {
        #[arg()]
        containerfile: String,
    },
}

pub fn setup_tera(recipe: String, containerfile: Option<PathBuf>) -> Result<(Tera, Context)> {
    let recipe_de =
        serde_yaml::from_str::<Recipe>(fs::read_to_string(PathBuf::from(&recipe))?.as_str())?
            .process_repos();

    let mut context = Context::from_serialize(recipe_de)?;
    context.insert("recipe", &recipe);

    let mut tera = Tera::default();
    match containerfile {
        Some(containerfile) => {
            tera.add_raw_template("Containerfile", &read_to_string(containerfile)?)?
        }
        None => tera.add_raw_template("Containerfile", DEFAULT_CONTAINERFILE)?,
    }
    tera.register_function(
        "print_containerfile",
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
    );
    tera.register_function(
        "print_autorun_scripts",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            match args.get("mode") {
                Some(v) => match from_value::<String>(v.clone()) {
                    Ok(mode) if mode == "pre" || mode == "post" => {
                        Ok(read_dir(format!("scripts/{mode}"))?
                            .fold(String::from(""), |mut acc: String, script| match script {
                                Ok(entry) => {
                                    let file_name = entry.file_name();
                                    if let Some(file_name) = file_name.to_str() {
                                        if file_name.ends_with(".sh") {
                                            acc += format!(
                                                "RUN /tmp/scripts/{mode}/{file_name} {mode}\n"
                                            )
                                            .as_str();
                                        }
                                    }
                                    acc
                                }
                                Err(_) => acc,
                            })
                            .into())
                    }
                    _ => Err("Mode must be pre/post".into()),
                },
                None => Err("Need arg 'mode' set with 'pre' or 'post'".into()),
            }
        },
    );

    Ok((tera, context))
}
