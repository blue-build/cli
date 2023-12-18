//! The root library for ublue-rs.
//!
//! This module consists of the args for the cli as well as the
//! initial entrypoint for setting up tera to properly template
//! the Containerfile. There is support for legacy starting point
//! recipes using the feature flag 'legacy' and support for the newest
//! starting point setup using the 'modules' feature flag. You will not want
//! to use both features at the same time. For now the 'legacy' feature
//! is the default feature until modules works 1-1 with ublue starting point.

#[cfg(feature = "init")]
pub mod init;

#[cfg(feature = "build")]
pub mod build;

pub mod module_recipe;

use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    path::{Path, PathBuf},
};

use anyhow::Result;
use module_recipe::Recipe;
use tera::{Context, Tera};

pub const DEFAULT_CONTAINERFILE: &str = include_str!("../templates/Containerfile.tera");

fn setup_tera(recipe: &Path, containerfile: Option<&PathBuf>) -> Result<(Tera, Context)> {
    let recipe_de = serde_yaml::from_str::<Recipe>(fs::read_to_string(recipe)?.as_str())?;

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
                        "config/containerfiles/{containerfile}/Containerfile"
                    ))?
                    .into()),
                    None => Err("Arg containerfile wasn't a string".into()),
                },
                None => Err("Needs the argument 'containerfile'".into()),
            }
        },
    );

    tera.register_function(
        "print_module_context",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            match args.get("module") {
                Some(v) => Ok(match serde_json::to_string(v) {
                    Ok(s) => s,
                    Err(_) => "Unable to serialize".into(),
                }
                .into()),
                None => Err("Needs the argument 'module'".into()),
            }
        },
    );

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

    Ok((tera, context))
}

pub fn template_file(
    recipe: &Path,
    containerfile: Option<&PathBuf>,
    output: Option<&PathBuf>,
) -> Result<()> {
    let (tera, context) = setup_tera(recipe, containerfile)?;
    let output_str = tera.render("Containerfile", &context)?;

    if let Some(output) = output {
        std::fs::write(output, output_str)?;
    } else {
        println!("{output_str}");
    }
    Ok(())
}
