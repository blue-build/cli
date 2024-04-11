use std::{borrow::Cow, env, fs, path::Path, process};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::{
    CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_SERVER_HOST, CI_SERVER_PROTOCOL, CONFIG_PATH,
    COSIGN_PATH, FILES_PATH, GITHUB_RESPOSITORY, GITHUB_SERVER_URL,
};
use log::{debug, error, trace, warn};
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

    #[builder(setter(into))]
    os_version: Cow<'a, str>,

    #[builder(setter(into))]
    registry: Cow<'a, str>,
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

fn files_dir_exists() -> bool {
    let path = Path::new(FILES_PATH);
    let exists = path.exists() && path.is_dir();

    if !exists {
        warn!("Use of the {CONFIG_PATH} directory is deprecated. Please move your non-recipe files into {FILES_PATH}");
    }

    exists
}

mod filters {
    #[allow(clippy::unnecessary_wraps)]
    pub fn replace<T: std::fmt::Display>(input: T, from: char, to: &str) -> askama::Result<String> {
        Ok(format!("{input}").replace(from, to))
    }
}
