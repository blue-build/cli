use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::Result;
use askama::Template;
use chrono::Local;
use clap::Args;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe,
    recipe_path: &'a Path,

    #[builder(default)]
    export_script: ExportsTemplate,
}

#[derive(Debug, Clone, Default, Template)]
#[template(path = "export.sh", escape = "none")]
pub struct ExportsTemplate;

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    pub description: String,

    #[serde(alias = "base-image")]
    pub base_image: String,

    #[serde(alias = "image-version")]
    pub image_version: String,

    #[serde(flatten)]
    pub modules_ext: ModuleExt,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Recipe {
    pub fn generate_tags(&self) -> Vec<String> {
        debug!("Generating image tags for {}", &self.name);
        trace!("Recipe::generate_tags()");

        let mut tags: Vec<String> = Vec::new();
        let image_version = &self.image_version;
        let timestamp = Local::now().format("%Y%m%d").to_string();

        if let (Ok(commit_branch), Ok(default_branch), Ok(commit_sha), Ok(pipeline_source)) = (
            env::var("CI_COMMIT_REF_NAME"),
            env::var("CI_DEFAULT_BRANCH"),
            env::var("CI_COMMIT_SHORT_SHA"),
            env::var("CI_PIPELINE_SOURCE"),
        ) {
            trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_DEFAULT_BRANCH={default_branch},CI_COMMIT_SHORT_SHA={commit_sha}, CI_PIPELINE_SOURCE={pipeline_source}");
            warn!("Detected running in Gitlab, pulling information from CI variables");

            if let Ok(mr_iid) = env::var("CI_MERGE_REQUEST_IID") {
                trace!("CI_MERGE_REQUEST_IID={mr_iid}");
                if pipeline_source == "merge_request_event" {
                    debug!("Running in a MR");
                    tags.push(format!("mr-{mr_iid}-{image_version}"));
                }
            }

            if default_branch != commit_branch {
                debug!("Running on branch {commit_branch}");
                tags.push(format!("{commit_branch}-{image_version}"));
            } else {
                debug!("Running on the default branch");
                tags.push(image_version.to_string());
                tags.push(format!("{image_version}-{timestamp}"));
                tags.push(timestamp.to_string());
            }

            tags.push(format!("{commit_sha}-{image_version}"));
        } else if let (
            Ok(github_event_name),
            Ok(github_event_number),
            Ok(github_sha),
            Ok(github_ref_name),
        ) = (
            env::var("GITHUB_EVENT_NAME"),
            env::var("PR_EVENT_NUMBER"),
            env::var("GITHUB_SHA"),
            env::var("GITHUB_REF_NAME"),
        ) {
            trace!("GITHUB_EVENT_NAME={github_event_name},PR_EVENT_NUMBER={github_event_number},GITHUB_SHA={github_sha},GITHUB_REF_NAME={github_ref_name}");
            warn!("Detected running in Github, pulling information from GITHUB variables");

            let mut short_sha = github_sha.clone();
            short_sha.truncate(7);

            if github_event_name == "pull_request" {
                debug!("Running in a PR");
                tags.push(format!("pr-{github_event_number}-{image_version}"));
            } else if github_ref_name == "live" {
                tags.push(image_version.to_owned());
                tags.push(format!("{image_version}-{timestamp}"));
                tags.push("latest".to_string());
            } else {
                tags.push(format!("br-{github_ref_name}-{image_version}"));
            }
            tags.push(format!("{short_sha}-{image_version}"));
        } else {
            warn!("Running locally");
            tags.push(format!("{image_version}-local"));
        }
        info!("Finished generating tags!");
        debug!("Tags: {tags:#?}");
        tags
    }
}

#[derive(Serialize, Clone, Deserialize, Debug, Template)]
#[template(path = "Containerfile.module", escape = "none")]
pub struct ModuleExt {
    pub modules: Vec<Module>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Module {
    #[serde(rename = "type")]
    pub module_type: Option<String>,

    #[serde(rename = "from-file")]
    pub from_file: Option<String>,

    #[serde(flatten)]
    pub config: HashMap<String, Value>,
}

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

        debug!("Deserializing recipe");
        let recipe_de = serde_yaml::from_str::<Recipe>(fs::read_to_string(&self.recipe)?.as_str())?;
        trace!("recipe_de: {recipe_de:#?}");

        let template = ContainerFileTemplate::builder()
            .recipe(&recipe_de)
            .recipe_path(&self.recipe)
            .build();

        let output_str = template.render()?;

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
}

fn print_script(script_contents: &ExportsTemplate) -> String {
    trace!("print_script({script_contents})");

    format!(
        "\"{}\"",
        script_contents
            .render()
            .unwrap_or_else(|e| {
                error!("Failed to render export.sh script: {e}");
                process::exit(1);
            })
            .replace('\n', "\\n")
            .replace('\"', "\\\"")
            .replace('$', "\\$")
    )
}

fn get_containerfile_list(module: &Module) -> Option<Vec<String>> {
    if module.module_type.as_ref()? == "containerfile" {
        Some(
            module
                .config
                .get("containerfiles")?
                .as_sequence()?
                .iter()
                .filter_map(|t| Some(t.as_str()?.to_owned()))
                .collect(),
        )
    } else {
        None
    }
}

fn print_containerfile(containerfile: &str) -> String {
    trace!("print_containerfile({containerfile})");
    debug!("Loading containerfile contents for {containerfile}");

    let path = format!("config/containerfiles/{containerfile}/Containerfile");

    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
        error!("Failed to read file {path}: {e}");
        process::exit(1);
    });

    trace!("Containerfile contents {path}:\n{file}");

    file
}

fn get_module_from_file(file: &str) -> ModuleExt {
    trace!("get_module_from_file({file})");

    serde_yaml::from_str(
        fs::read_to_string(format!("config/{file}").as_str())
            .unwrap_or_else(|e| {
                error!("Failed to read module {file}: {e}");
                process::exit(1);
            })
            .as_str(),
    )
    .unwrap_or_else(|e| {
        error!("Failed to parse {file}: {e}");
        process::exit(1);
    })
}

fn print_module_context(module: &Module) -> String {
    serde_json::to_string(module).unwrap_or_else(|e| {
        error!("Failed to parse module: {e}");
        process::exit(1);
    })
}

fn running_gitlab_actions() -> bool {
    trace!(" running_gitlab_actions()");

    env::var("GITHUB_ACTIONS").is_ok_and(|e| e == "true")
}
