use std::{collections::HashMap, env};

use chrono::Local;
use log::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    pub description: String,

    #[serde(alias = "base-image")]
    pub base_image: String,

    #[serde(alias = "image-version")]
    pub image_version: u16,

    pub modules: Vec<Module>,

    pub containerfiles: Option<Containerfiles>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Recipe {
    pub fn generate_tags(&self) -> Vec<String> {
        debug!("Generating image tags for {}", &self.name);
        trace!("Recipe::generate_tags()");

        let mut tags: Vec<String> = Vec::new();
        let image_version = self.image_version;
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
                tags.push(format!("{image_version}"));
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Module {
    #[serde(rename = "type")]
    pub module_type: Option<String>,

    #[serde(rename = "from-file")]
    pub from_file: Option<String>,

    #[serde(flatten)]
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Containerfiles {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}
