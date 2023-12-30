use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::Result;
use clap::Args;
use log::{debug, error, info, trace};
use tera::{Context, Tera};
use typed_builder::TypedBuilder;

use crate::module_recipe::Recipe;

pub const DEFAULT_CONTAINERFILE: &str = include_str!("../templates/Containerfile.tera");

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct TemplateCommand {
    /// The recipe file to create a template from
    #[arg()]
    recipe: PathBuf,

    /// Optional Containerfile to use as a template
    #[arg(short, long)]
    #[builder(default, setter(into))]
    containerfile: Option<PathBuf>,

    /// File to output to instead of STDOUT
    #[arg(short, long)]
    #[builder(default, setter(into))]
    output: Option<PathBuf>,
}

impl TemplateCommand {
    pub fn try_run(&self) -> Result<()> {
        info!("Templating for recipe at {}", self.recipe.display());

        self.template_file()
    }

    pub fn run(&self) {
        if let Err(e) = self.try_run() {
            error!("Failed to template file: {e}");
            process::exit(1);
        }
    }

    fn template_file(&self) -> Result<()> {
        trace!("TemplateCommand::template_file()");

        debug!("Setting up tera");
        let (tera, context) = self.setup_tera()?;

        trace!("tera: {tera:#?}");
        trace!("context: {context:#?}");

        debug!("Rendering Containerfile");
        let output_str = tera.render("Containerfile", &context)?;

        match self.output.as_ref() {
            Some(output) => {
                debug!("Templating to file {}", output.display());
                trace!("Containerfile:\n{output_str}");

                std::fs::write(output, output_str)?;
            }
            None => {
                debug!("Templating to stdout");
                println!("{output_str}");
            }
        }

        info!("Finished templating Containerfile");
        Ok(())
    }

    fn setup_tera(&self) -> Result<(Tera, Context)> {
        trace!("TemplateCommand::setup_tera()");

        debug!("Deserializing recipe");
        let recipe_de = serde_yaml::from_str::<Recipe>(fs::read_to_string(&self.recipe)?.as_str())?;
        trace!("recipe_de: {recipe_de:#?}");

        debug!("Building context");
        let mut context = Context::from_serialize(recipe_de)?;

        trace!("add to context 'recipe': {}", self.recipe.display());
        context.insert("recipe", &self.recipe);

        let mut tera = Tera::default();

        match self.containerfile.as_ref() {
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

                            let path =
                                format!("config/containerfiles/{containerfile}/Containerfile");
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

        debug!("Registering function `running_gitlab_actions`");
        tera.register_function(
            "running_gitlab_actions",
            |_: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
                trace!("tera fn running_gitlab_actions()");

                Ok(env::var("GITHUB_ACTIONS").is_ok_and(|e| e == "true").into())
            },
        );

        Ok((tera, context))
    }
}
