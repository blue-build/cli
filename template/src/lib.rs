use std::{borrow::Cow, env, fs, path::Path, process};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::*;
use log::{debug, error, trace};
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub use askama::Template;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile.j2", escape = "none")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe<'a>,

    #[builder(setter(into))]
    recipe_path: &'a Path,

    #[builder(setter(into))]
    build_id: Uuid,

    #[builder(default)]
    export_script: ExportsTemplate,

    #[builder(setter(into))]
    os_version: Cow<'a, str>,
}

#[derive(Debug, Clone, Default, Template)]
#[template(path = "export.sh", escape = "none")]
pub struct ExportsTemplate;

impl ExportsTemplate {
    fn print_script(&self) -> String {
        trace!("print_script({self})");

        format!(
            "\"{}\"",
            self.render()
                .unwrap_or_else(|e| {
                    error!("Failed to render export.sh script: {e}");
                    process::exit(1);
                })
                .replace('\n', "\\n")
                .replace('\"', "\\\"")
                .replace('$', "\\$")
        )
    }
}

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "github_issue.j2", escape = "md")]
pub struct GithubIssueTemplate<'a> {
    #[builder(setter(into))]
    bb_version: Cow<'a, str>,

    #[builder(setter(into))]
    build_rust_channel: Cow<'a, str>,

    #[builder(setter(into))]
    build_time: Cow<'a, str>,

    #[builder(setter(into))]
    git_commit_hash: Cow<'a, str>,

    #[builder(setter(into))]
    os_name: Cow<'a, str>,

    #[builder(setter(into))]
    os_version: Cow<'a, str>,

    #[builder(setter(into))]
    pkg_branch_tag: Cow<'a, str>,

    #[builder(setter(into))]
    recipe: Cow<'a, str>,

    #[builder(setter(into))]
    rust_channel: Cow<'a, str>,

    #[builder(setter(into))]
    rust_version: Cow<'a, str>,

    #[builder(setter(into))]
    shell_name: Cow<'a, str>,

    #[builder(setter(into))]
    shell_version: Cow<'a, str>,

    #[builder(setter(into))]
    terminal_name: Cow<'a, str>,

    #[builder(setter(into))]
    terminal_version: Cow<'a, str>,
}

fn has_cosign_file() -> bool {
    trace!("has_cosign_file()");
    std::env::current_dir()
        .map(|p| p.join(COSIGN_PATH).exists())
        .unwrap_or(false)
}

#[must_use]
fn print_containerfile(containerfile: &str) -> String {
    trace!("print_containerfile({containerfile})");
    debug!("Loading containerfile contents for {containerfile}");

    let path = format!("config/containerfiles/{containerfile}/Containerfile");

    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
        error!("Failed to read file {path}: {e}");
        process::exit(1);
    });

    debug!("Containerfile contents {path}:\n{file}");

    file
}

fn get_github_repo_owner() -> Option<String> {
    Some(env::var(GITHUB_REPOSITORY_OWNER).ok()?.to_lowercase())
}

fn get_gitlab_registry_path() -> Option<String> {
    Some(
        format!(
            "{}/{}/{}",
            env::var(CI_REGISTRY).ok()?,
            env::var(CI_PROJECT_NAMESPACE).ok()?,
            env::var(CI_PROJECT_NAME).ok()?,
        )
        .to_lowercase(),
    )
}

fn get_repo_url() -> Option<String> {
    Some(
        match (
            // GitHub vars
            env::var(GITHUB_SERVER_URL),
            env::var(GITHUB_RESPOSITORY),
            // GitLab vars
            env::var(CI_SERVER_PROTOCOL),
            env::var(CI_SERVER_HOST),
            env::var(CI_PROJECT_NAMESPACE),
            env::var(CI_PROJECT_NAME),
        ) {
            (Ok(github_server), Ok(github_repo), _, _, _, _) => {
                format!("{github_server}/{github_repo}")
            }
            (
                _,
                _,
                Ok(ci_server_protocol),
                Ok(ci_server_host),
                Ok(ci_project_namespace),
                Ok(ci_project_name),
            ) => {
                format!(
                    "{ci_server_protocol}://{ci_server_host}/{ci_project_namespace}/{ci_project_name}"
                )
            }
            _ => return None,
        },
    )
}

fn modules_exists() -> bool {
    let mod_path = Path::new("modules");
    mod_path.exists() && mod_path.is_dir()
}
