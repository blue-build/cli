use std::{borrow::Cow, env, fs, path::PathBuf, process};

use askama::Template;
use chrono::Local;
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

#[derive(Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct Recipe<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    #[builder(setter(into))]
    pub description: Cow<'a, str>,

    #[serde(alias = "base-image")]
    #[builder(setter(into))]
    pub base_image: Cow<'a, str>,

    #[serde(alias = "image-version")]
    #[builder(setter(into))]
    pub image_version: Cow<'a, str>,

    #[serde(alias = "blue-build-tag")]
    #[builder(default, setter(into, strip_option))]
    pub blue_build_tag: Option<Cow<'a, str>>,

    #[serde(flatten)]
    pub modules_ext: ModuleExt<'a>,

    #[serde(flatten)]
    #[builder(setter(into))]
    pub extra: IndexMap<String, Value>,
}

impl<'a> Recipe<'a> {
    #[must_use]
    pub fn generate_tags(&self) -> Vec<String> {
        trace!("Recipe::generate_tags()");
        debug!("Generating image tags for {}", &self.name);

        let mut tags: Vec<String> = Vec::new();
        let image_version = self.image_version.as_ref();
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
                tags.push(image_version.to_string());
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

#[derive(Serialize, Clone, Deserialize, Debug, Template, TypedBuilder)]
#[template(path = "Containerfile.module", escape = "none")]
pub struct ModuleExt<'a> {
    #[builder(default, setter(into))]
    pub modules: Cow<'a, [Module<'a>]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Module<'a> {
    #[serde(rename = "type")]
    #[builder(default, setter(into, strip_option))]
    pub module_type: Option<Cow<'a, str>>,

    #[serde(rename = "from-file")]
    #[builder(default, setter(into, strip_option))]
    pub from_file: Option<Cow<'a, str>>,

    #[serde(flatten)]
    #[builder(default, setter(into))]
    pub config: IndexMap<String, Value>,
}

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //

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

fn get_module_from_file(file_name: &str) -> String {
    trace!("get_module_from_file({file_name})");

    let io_err_fn = |e| {
        error!("Failed to read module {file_name}: {e}");
        process::exit(1);
    };

    let file_path = PathBuf::from("config").join(file_name);

    let file = fs::read_to_string(file_path).unwrap_or_else(io_err_fn);

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
            let module = serde_yaml::from_str::<Module>(file.as_str()).unwrap_or_else(serde_err_fn);

            ModuleExt::builder()
                .modules(vec![module])
                .build()
                .render()
                .unwrap_or_else(template_err_fn)
        },
        |module_ext| module_ext.render().unwrap_or_else(template_err_fn),
    )
}

fn print_module_context(module: &Module) -> String {
    serde_json::to_string(module).unwrap_or_else(|e| {
        error!("Failed to parse module: {e}");
        process::exit(1);
    })
}
