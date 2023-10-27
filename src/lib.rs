//! The root library for ublue-rs.
//!
//! This module consists of the args for the cli as well as the
//! initial entrypoint for setting up tera to properly template
//! the Containerfile. There is support for legacy starting point
//! recipes using the feature flag 'legacy' and support for the newest
//! starting point setup using the 'modules' feature flag. You will not want
//! to use both features at the same time. For now the 'legacy' feature
//! is the default feature until modules works 1-1 with ublue starting point.

#[cfg(all(feature = "legacy", feature = "modules"))]
compile_error!("Both 'legacy' and 'modules' features cannot be used at the same time.");

#[cfg(feature = "init")]
pub mod init;

#[cfg(feature = "legacy")]
pub mod recipe;

#[cfg(feature = "modules")]
pub mod module_recipe;

use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    path::PathBuf,
};

use anyhow::Result;
use cfg_if;
use clap::{Parser, Subcommand};
use tera::{Context, Tera};

cfg_if::cfg_if! {
    if #[cfg(feature = "legacy")] {
        use recipe::Recipe;
        use std::fs::read_dir;
        pub const DEFAULT_CONTAINERFILE: &str = include_str!("../templates/Containerfile.legacy");
    } else if #[cfg(feature = "modules")] {
        use module_recipe::Recipe;
        pub const DEFAULT_CONTAINERFILE: &str = include_str!("../templates/Containerfile.modules");
    }
}

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

    /// Initialize a new Ublue Starting Point repo
    #[cfg(feature = "init")]
    Init {
        /// The directory to extract the files into. Defaults to the current directory
        #[arg()]
        dir: Option<PathBuf>,
    },

    /// Build an image from a Containerfile
    #[cfg(feature = "build")]
    Build {
        #[arg()]
        containerfile: String,
    },
}

pub fn setup_tera(recipe: String, containerfile: Option<PathBuf>) -> Result<(Tera, Context)> {
    let recipe_de =
        serde_yaml::from_str::<Recipe>(fs::read_to_string(PathBuf::from(&recipe))?.as_str())?;

    #[cfg(feature = "legacy")]
    let recipe_de = recipe_de.process_repos();

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
                Some(v) => match v.as_str() {
                    Some(containerfile) => Ok(read_to_string(format!(
                        "containerfiles/{containerfile}/Containerfile"
                    ))?
                    .into()),
                    None => Err("Arg containerfile wasn't a string".into()),
                },
                None => Err("Needs the argument 'containerfile'".into()),
            }
        },
    );

    #[cfg(feature = "modules")]
    tera.register_function(
        "print_module_context",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            match args.get("module") {
                Some(v) => Ok(match serde_yaml::to_string(v) {
                    Ok(s) => s.escape_default().collect::<String>(),
                    Err(_) => "Unable to serialize".into(),
                }
                .into()),
                None => Err("Needs the argument 'module'".into()),
            }
        },
    );

    #[cfg(feature = "modules")]
    tera.register_function(
        "get_module_from_file",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            match args.get("file") {
                Some(v) => {
                    let file = match v.as_str() {
                        Some(s) => s,
                        None => return Err("Property 'from-file' must be a string".into()),
                    };
                    match serde_yaml::from_str::<tera::Value>(
                        read_to_string(format!("config/{file}"))?.as_str(),
                    ) {
                        Ok(context) => Ok(context),
                        Err(_) => Err(format!("Unable to deserialize file {file}").into()),
                    }
                }
                None => Err("Needs the argument 'file'".into()),
            }
        },
    );

    #[cfg(feature = "legacy")]
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
