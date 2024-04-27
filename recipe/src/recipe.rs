use std::{borrow::Cow, env, fs, path::Path};

use anyhow::{Context, Result};
use blue_build_utils::constants::{
    CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
    CI_PIPELINE_SOURCE, GITHUB_EVENT_NAME, GITHUB_REF_NAME, GITHUB_SHA, PR_EVENT_NUMBER,
};
use chrono::Local;
use indexmap::IndexMap;
use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::{Module, ModuleExt};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
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

    #[serde(alias = "blue-build-tag", skip_serializing_if = "Option::is_none")]
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
    pub fn generate_tags(&self, os_version: u64) -> Vec<String> {
        trace!("Recipe::generate_tags()");
        trace!("Generating image tags for {}", &self.name);

        let mut tags: Vec<String> = Vec::new();
        let timestamp = Local::now().format("%Y%m%d").to_string();

        if let (Ok(commit_branch), Ok(default_branch), Ok(commit_sha), Ok(pipeline_source)) = (
            env::var(CI_COMMIT_REF_NAME),
            env::var(CI_DEFAULT_BRANCH),
            env::var(CI_COMMIT_SHORT_SHA),
            env::var(CI_PIPELINE_SOURCE),
        ) {
            trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_DEFAULT_BRANCH={default_branch},CI_COMMIT_SHORT_SHA={commit_sha}, CI_PIPELINE_SOURCE={pipeline_source}");
            warn!("Detected running in Gitlab, pulling information from CI variables");

            if let Ok(mr_iid) = env::var(CI_MERGE_REQUEST_IID) {
                trace!("CI_MERGE_REQUEST_IID={mr_iid}");
                if pipeline_source == "merge_request_event" {
                    debug!("Running in a MR");
                    tags.push(format!("mr-{mr_iid}-{os_version}"));
                }
            }

            if default_branch == commit_branch {
                debug!("Running on the default branch");
                tags.push(os_version.to_string());
                tags.push(format!("{timestamp}-{os_version}"));
                tags.push("latest".into());
                tags.push(timestamp);
            } else {
                debug!("Running on branch {commit_branch}");
                tags.push(format!("br-{commit_branch}-{os_version}"));
            }

            tags.push(format!("{commit_sha}-{os_version}"));
        } else if let (
            Ok(github_event_name),
            Ok(github_event_number),
            Ok(github_sha),
            Ok(github_ref_name),
        ) = (
            env::var(GITHUB_EVENT_NAME),
            env::var(PR_EVENT_NUMBER),
            env::var(GITHUB_SHA),
            env::var(GITHUB_REF_NAME),
        ) {
            trace!("GITHUB_EVENT_NAME={github_event_name},PR_EVENT_NUMBER={github_event_number},GITHUB_SHA={github_sha},GITHUB_REF_NAME={github_ref_name}");
            warn!("Detected running in Github, pulling information from GITHUB variables");

            let mut short_sha = github_sha;
            short_sha.truncate(7);

            if github_event_name == "pull_request" {
                debug!("Running in a PR");
                tags.push(format!("pr-{github_event_number}-{os_version}"));
            } else if github_ref_name == "live" || github_ref_name == "main" {
                tags.push(os_version.to_string());
                tags.push(format!("{timestamp}-{os_version}"));
                tags.push("latest".into());
                tags.push(timestamp);
            } else {
                tags.push(format!("br-{github_ref_name}-{os_version}"));
            }
            tags.push(format!("{short_sha}-{os_version}"));
        } else {
            warn!("Running locally");
            tags.push(format!("local-{os_version}"));
        }
        debug!("Finished generating tags!");
        debug!("Tags: {tags:#?}");

        tags.into_iter().map(|t| t.replace('/', "_")).collect()
    }

    /// # Parse a recipe file
    /// #
    /// # Errors
    pub fn parse<P: AsRef<Path>>(path: &P) -> Result<Self> {
        trace!("Recipe::parse({})", path.as_ref().display());

        let file_path = if Path::new(path.as_ref()).is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            std::env::current_dir()?.join(path.as_ref())
        };

        let file = fs::read_to_string(&file_path)
            .context(format!("Failed to read {}", file_path.display()))?;

        debug!("Recipe contents: {file}");

        let mut recipe = serde_yaml::from_str::<Recipe>(&file)
            .map_err(blue_build_utils::serde_yaml_err(&file))?;

        recipe.modules_ext.modules = Module::get_modules(&recipe.modules_ext.modules)?.into();

        Ok(recipe)
    }
}
