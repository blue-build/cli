use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

use askama::Template;
use chrono::Local;
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::commands::template::{get_containerfile_list, print_containerfile};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder, Template)]
#[template(path = "recipe/base", escape = "none")]

pub struct Recipe {
    #[builder(setter(into))]
    pub name: String,

    #[builder(setter(into))]
    pub description: String,

    #[serde(alias = "base-image")]
    #[builder(setter(into))]
    pub base_image: String,

    #[serde(alias = "image-version")]
    #[builder(setter(into))]
    pub image_version: String,

    #[serde(alias = "blue-build-tag")]
    #[builder(default, setter(into, strip_option))]
    pub blue_build_tag: Option<String>,

    #[serde(flatten)]
    pub modules_ext: ModuleExt,

    #[serde(flatten)]
    #[builder(setter(into))]
    pub extra: IndexMap<String, Value>,
}

impl Recipe {
    pub fn parse<P: AsRef<Path>>(recipe_path: &P) -> anyhow::Result<Self> {
        let recipe_path_string = recipe_path.as_ref().display().to_string();

        trace!("Recipe::parse_recipe({recipe_path_string})");
        debug!("Parsing recipe at {recipe_path_string}");

        let file = fs::read_to_string(recipe_path).unwrap_or_else(|e| {
            error!("Failed to read file {recipe_path_string}: {e}");
            process::exit(1);
        });

        trace!("Recipe contents {recipe_path_string}:\n{file}");

        serde_yaml::from_str::<Recipe>(file.as_str()).map_err(|e| {
            error!("Failed to parse recipe {recipe_path_string}: {e}");
            process::exit(1);
        })
    }

    #[must_use]
    pub fn generate_tags(&self) -> Vec<String> {
        trace!("Recipe::generate_tags()");
        debug!("Generating image tags for {}", &self.name);

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

            if default_branch == commit_branch {
                debug!("Running on the default branch");
                tags.push(image_version.to_string());
                tags.push(format!("{image_version}-{timestamp}"));
                tags.push(timestamp);
            } else {
                debug!("Running on branch {commit_branch}");
                tags.push(format!("{commit_branch}-{image_version}"));
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

            let mut short_sha = github_sha;
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

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct ModuleExt {
    #[builder(default, setter(into))]
    pub modules: Vec<Module>,
}

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Module {
    #[serde(rename = "type")]
    #[builder(default, setter(into, strip_option))]
    pub module_type: Option<String>,

    #[serde(rename = "from-file")]
    #[builder(default, setter(into, strip_option))]
    pub from_file: Option<String>,

    #[serde(flatten)]
    #[builder(default, setter(into))]
    pub config: IndexMap<String, Value>,
}

fn print_files(module: &Module) -> String {
    let mut files = String::new();
    for file in module.config.iter() {
        if let Some(sequence) = file.1.as_sequence() {
            for seq in sequence {
                let mapping = seq.as_mapping().unwrap();
                mapping.iter().for_each(|(k, v)| {
                    files.push_str(&format!(
                        "COPY {} => {}\n",
                        k.as_str().unwrap(),
                        v.as_str().unwrap()
                    ));
                });
            }
        }
    }
    files
}

pub fn get_module_from_file(file_name: &str) -> String {
    trace!("get_module_from_file({file_name})");

    let file_path = PathBuf::from("config").join(file_name);
    let file = fs::read_to_string(file_path).unwrap_or_else(|e| {
        error!("Failed to read module {file_name}: {e}");
        String::default()
    });

    let serde_err_fn = |e| {
        error!("Failed to deserialize module {file_name}: {e}");
        process::exit(1);
    };

    let template_err_fn = |e| {
        error!("Failed to render module {file_name}: {e}");
        process::exit(1);
    };

    serde_yaml::from_str::<ModuleExt>(file.as_str()).map_or_else(
        |_| {
            let module = serde_yaml::from_str::<Module>(file.as_str()).unwrap_or_else(|e| {
                error!("Failed to deserialize module {file_name}: {e}");
                process::exit(1);
            });

            ModuleTemplate::builder()
                .module_ext(&ModuleExt::builder().modules(vec![module]).build())
                .build()
                .render()
                .unwrap_or_else(template_err_fn)
        },
        |module_ext| {
            ModuleTemplate::builder()
                .module_ext(&module_ext)
                .build()
                .render()
                .unwrap_or_else(template_err_fn)
        },
    )
}
// pub fn parse_modules(file_name: &str) -> String {
//     trace!("parse_modules({file_name})");

//     let file_path = PathBuf::from("config").join(file_name);
//     let file = fs::read_to_string(file_path).unwrap_or_else(|e| {
//         error!("Failed to read module {file_name}: {e}");
//         String::default()
//     });

//     let template_err_fn = |e| {
//         error!("Failed to render module {file_name}: {e}");
//         process::exit(1);
//     };

//     serde_yaml::from_str::<Module>(file.as_str()).map_or_else(
//         |_| "".to_owned(),
//         |module| module.render().unwrap_or_else(template_err_fn),
//     )
// }
