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

use crate::{Module, ModuleExt, Stage, StagesExt};

/// The build recipe.
///
/// This is the top-level section of a recipe.yml.
/// This will contain information on the image and its
/// base image to assist with building the Containerfile
/// and tagging the image appropriately.
#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct Recipe<'a> {
    /// The name of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.title` label.
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    /// The description of the user's image.
    ///
    /// This will be set on the `org.opencontainers.image.description` label.
    #[builder(setter(into))]
    pub description: Cow<'a, str>,

    /// The base image from which to build the user's image.
    #[serde(alias = "base-image")]
    #[builder(setter(into))]
    pub base_image: Cow<'a, str>,

    /// The version/tag of the base image.
    #[serde(alias = "image-version")]
    #[builder(setter(into))]
    pub image_version: Cow<'a, str>,

    /// The version of `bluebuild` to install in the image
    #[serde(alias = "blue-build-tag", skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into, strip_option))]
    pub blue_build_tag: Option<Cow<'a, str>>,

    /// Alternate tags to the `latest` tag to add to the image.
    ///
    /// If `alt-tags` is not supplied by the user, the build system
    /// will assume `latest` and will also tag with the
    /// timestamp with no version (e.g. `20240429`).
    ///
    /// Any user input will override the `latest` and timestamp tags.
    #[serde(alias = "alt-tags", skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(into, strip_option))]
    pub alt_tags: Option<Vec<Cow<'a, str>>>,

    /// The stages extension of the recipe.
    ///
    /// This hold the list of stages that can
    /// be used to build software outside of
    /// the final build image.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub stages_ext: Option<StagesExt<'a>>,

    /// The modules extension of the recipe.
    ///
    /// This holds the list of modules to be run on the image.
    #[serde(flatten)]
    pub modules_ext: ModuleExt<'a>,

    /// Extra data that the user might have added. This is
    /// done in case we serialize the data to a yaml file
    /// so that we retain any unused information.
    #[serde(flatten)]
    #[builder(setter(into))]
    pub extra: IndexMap<String, Value>,
}

impl<'a> Recipe<'a> {
    /// Generate a list of tags based on the OS version.
    ///
    /// ## CI
    /// The tags are generated based on the CI system that
    /// is detected. The general format for the default branch is:
    /// - `${os_version}`
    /// - `${timestamp}-${os_version}`
    ///
    /// On a branch:
    /// - `br-${branch_name}-${os_version}`
    ///
    /// In a PR(GitHub)/MR(GitLab)
    /// - `pr-${pr_event_number}-${os_version}`/`mr-${mr_iid}-${os_version}`
    ///
    /// In all above cases the short git sha is also added:
    /// - `${commit_sha}-${os_version}`
    ///
    /// When `alt_tags` are not present, the following tags are added:
    /// - `latest`
    /// - `${timestamp}`
    ///
    /// ## Locally
    /// When ran locally, only a local tag is created:
    /// - `local-${os_version}`
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

                if let Some(alt_tags) = self.alt_tags.as_ref() {
                    tags.extend(alt_tags.iter().map(ToString::to_string));
                } else {
                    tags.push("latest".into());
                    tags.push(timestamp);
                }
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

                if let Some(alt_tags) = self.alt_tags.as_ref() {
                    tags.extend(alt_tags.iter().map(ToString::to_string));
                } else {
                    tags.push("latest".into());
                    tags.push(timestamp);
                }
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

    /// Parse a recipe file
    ///
    /// # Errors
    /// Errors when a yaml file cannot be deserialized,
    /// or a linked module yaml file does not exist.
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

        recipe.modules_ext.modules = Module::get_modules(&recipe.modules_ext.modules, None)?.into();

        if let Some(ref mut stages_ext) = recipe.stages_ext {
            stages_ext.stages = Stage::get_stages(&stages_ext.stages, None)?.into();
        }

        Ok(recipe)
    }
}
