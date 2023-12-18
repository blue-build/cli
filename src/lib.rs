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
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use log::{debug, info, trace};
use module_recipe::Recipe;
use tera::{Context, Tera};

pub const DEFAULT_CONTAINERFILE: &str = include_str!("../templates/Containerfile.tera");

fn setup_tera(recipe: &Path, containerfile: Option<&PathBuf>) -> Result<(Tera, Context)> {
    trace!("setup_tera({recipe:?}, {containerfile:?})");

    debug!("Deserializing recipe");
    let recipe_de = serde_yaml::from_str::<Recipe>(fs::read_to_string(recipe)?.as_str())?;
    trace!("recipe_de: {recipe_de:#?}");

    debug!("Building context");
    let mut context = Context::from_serialize(recipe_de)?;

    trace!("add to context 'recipe': {recipe:?}");
    context.insert("recipe", &recipe);

    let mut tera = Tera::default();

    match containerfile {
        Some(containerfile) => {
            debug!("Using {} as the template", containerfile.display());
            tera.add_raw_template("Containerfile", &fs::read_to_string(containerfile)?)?
        }
        None => tera.add_raw_template("Containerfile", DEFAULT_CONTAINERFILE)?,
    }

    debug!("Registering function `print_containerfile`");
    tera.register_function(
        "print_containerfile",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            trace!("tera fn print_containerfile({args:#?})");
            match args.get("containerfile") {
                Some(v) => match v.as_str() {
                    Some(containerfile) => {
                        debug!("Loading containerfile contents for {containerfile}");

                        let path = format!("config/containerfiles/{containerfile}/Containerfile");
                        let path = Path::new(path.as_str());

                        let file = fs::read_to_string(path)?;

                        trace!("Containerfile contents {}:\n{file}", path.display());
                        Ok(file.into())
                    }
                    None => Err("Arg containerfile wasn't a string".into()),
                },
                None => Err("Needs the argument 'containerfile'".into()),
            }
        },
    );

    debug!("Registering function `print_module_context`");
    tera.register_function(
        "print_module_context",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            trace!("tera fn print_module_context({args:#?})");
            match args.get("module") {
                Some(v) => match serde_json::to_string(v) {
                    Ok(s) => Ok(s.into()),
                    Err(e) => Err(format!("Unable to serialize: {e}").into()),
                },
                None => Err("Needs the argument 'module'".into()),
            }
        },
    );

    debug!("Registering function `get_module_from_file`");
    tera.register_function(
        "get_module_from_file",
        |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            trace!("tera fn get_module_from_file({args:#?})");
            match args.get("file") {
                Some(v) => {
                    let file = match v.as_str() {
                        Some(s) => s,
                        None => return Err("Property 'from-file' must be a string".into()),
                    };

                    trace!("from-file: {file}");
                    match serde_yaml::from_str::<tera::Value>(
                        fs::read_to_string(format!("config/{file}"))?.as_str(),
                    ) {
                        Ok(context) => {
                            trace!("context: {context}");
                            Ok(context)
                        }
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
    trace!("template_file({recipe:?}, {containerfile:?}, {output:?})");

    debug!("Setting up tera");
    let (tera, context) = setup_tera(recipe, containerfile)?;

    trace!("tera: {tera:#?}");
    trace!("context: {context:#?}");

    debug!("Rendering Containerfile");
    let output_str = tera.render("Containerfile", &context)?;

    if let Some(output) = output {
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
