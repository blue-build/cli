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
        trace!("BuildCommand::generate_tags({self:#?})");

        let mut tags: Vec<String> = Vec::new();
        let image_version = self.image_version;
        let timestamp = Local::now().format("%Y%m%d").to_string();

        if env::var("CI").is_ok() {
            warn!("Detected running in Gitlab, pulling information from CI variables");

            if let (Ok(mr_iid), Ok(pipeline_source)) = (
                env::var("CI_MERGE_REQUEST_IID"),
                env::var("CI_PIPELINE_SOURCE"),
            ) {
                trace!("CI_MERGE_REQUEST_IID={mr_iid}, CI_PIPELINE_SOURCE={pipeline_source}");
                if pipeline_source == "merge_request_event" {
                    debug!("Running in a MR");
                    tags.push(format!("{mr_iid}-{image_version}"));
                }
            }

            if let Ok(commit_sha) = env::var("CI_COMMIT_SHORT_SHA") {
                trace!("CI_COMMIT_SHORT_SHA={commit_sha}");
                tags.push(format!("{commit_sha}-{image_version}"));
            }

            if let (Ok(commit_branch), Ok(default_branch)) = (
                env::var("CI_COMMIT_REF_NAME"),
                env::var("CI_DEFAULT_BRANCH"),
            ) {
                trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_DEFAULT_BRANCH={default_branch}");
                if default_branch != commit_branch {
                    debug!("Running on branch {commit_branch}");
                    tags.push(format!("br-{commit_branch}-{image_version}"));
                } else {
                    debug!("Running on the default branch");
                    tags.push(image_version.to_string());
                    tags.push(format!("{image_version}-{timestamp}"));
                    tags.push(timestamp.to_string());
                }
            }
        } else {
            warn!("Running locally");
            tags.push(format!("{image_version}-local"));
        }
        info!("Finished generating tags!");
        trace!("Tags: {tags:#?}");
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
