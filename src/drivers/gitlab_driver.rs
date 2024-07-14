use std::env;

use blue_build_utils::constants::{
    CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
    CI_PIPELINE_SOURCE, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_PROJECT_URL, CI_REGISTRY,
    CI_SERVER_HOST, CI_SERVER_PROTOCOL,
};
use log::{debug, trace};
use miette::{Context, IntoDiagnostic};

use crate::drivers::Driver;

use super::CiDriver;

pub struct GitlabDriver;

impl CiDriver for GitlabDriver {
    fn on_default_branch() -> bool {
        env::var(CI_DEFAULT_BRANCH).is_ok_and(|default_branch| {
            env::var(CI_COMMIT_REF_NAME).is_ok_and(|branch| default_branch == branch)
        })
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        Ok(format!(
            "{}//.gitlab-ci.yml@refs/heads/{}",
            env::var(CI_DEFAULT_BRANCH)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_DEFAULT_BRANCH}'"))?,
            env::var(CI_PROJECT_URL)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_PROJECT_URL}'"))?,
        ))
    }

    fn generate_tags(recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        let commit_branch = env::var(CI_COMMIT_REF_NAME)
            .into_diagnostic()
            .with_context(|| format!("Failed to get '{CI_COMMIT_REF_NAME}'"))?;
        let commit_sha = env::var(CI_COMMIT_SHORT_SHA)
            .into_diagnostic()
            .with_context(|| format!("Failed to get {CI_COMMIT_SHORT_SHA}'"))?;
        let pipeline_source = env::var(CI_PIPELINE_SOURCE)
            .into_diagnostic()
            .with_context(|| format!("Failed to get {CI_PIPELINE_SOURCE}'"))?;
        trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_COMMIT_SHORT_SHA={commit_sha}, CI_PIPELINE_SOURCE={pipeline_source}");

        let mut tags: Vec<String> = Vec::new();
        let os_version = Driver::get_os_version(recipe)?;

        if let Ok(mr_iid) = env::var(CI_MERGE_REQUEST_IID) {
            trace!("CI_MERGE_REQUEST_IID={mr_iid}");
            if pipeline_source == "merge_request_event" {
                debug!("Running in a MR");
                tags.push(format!("mr-{mr_iid}-{os_version}"));
            }
        }

        if Self::on_default_branch() {
            debug!("Running on the default branch");

            tags.push(os_version.to_string());

            let timestamp = blue_build_utils::get_tag_timestamp();
            tags.push(format!("{timestamp}-{os_version}"));

            if let Some(ref alt_tags) = recipe.alt_tags {
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
        Ok(tags)
    }

    fn get_repo_url() -> miette::Result<String> {
        Ok(format!(
            "{}://{}/{}/{}",
            env::var(CI_SERVER_PROTOCOL)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_SERVER_PROTOCOL}'"))?,
            env::var(CI_SERVER_HOST)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_SERVER_HOST}'"))?,
            env::var(CI_PROJECT_NAMESPACE)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_PROJECT_NAMESPACE}'"))?,
            env::var(CI_PROJECT_NAME)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_PROJECT_NAME}'"))?,
        ))
    }

    fn get_registry() -> miette::Result<String> {
        Ok(format!(
            "{}/{}/{}",
            env::var(CI_REGISTRY)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_REGISTRY}'"))?,
            env::var(CI_PROJECT_NAMESPACE)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_PROJECT_NAMESPACE}'"))?,
            env::var(CI_PROJECT_NAME)
                .into_diagnostic()
                .with_context(|| format!("Failed to get '{CI_PROJECT_NAME}'"))?,
        )
        .to_lowercase())
    }
}
