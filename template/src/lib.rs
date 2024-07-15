use std::{borrow::Cow, fs, path::Path, process};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::{
    CONFIG_PATH, CONTAINERFILES_PATH, CONTAINER_FILE, COSIGN_PUB_PATH, FILES_PATH,
};
use log::{debug, error, trace, warn};
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub use askama::Template;

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "Containerfile.j2", escape = "none", whitespace = "minimize")]
pub struct ContainerFileTemplate<'a> {
    recipe: &'a Recipe<'a>,

    #[builder(setter(into))]
    recipe_path: &'a Path,

    #[builder(setter(into))]
    build_id: Uuid,

    os_version: u64,

    #[builder(setter(into))]
    registry: Cow<'a, str>,

    #[builder(setter(into))]
    exports_tag: Cow<'a, str>,

    #[builder(setter(into))]
    repo: Cow<'a, str>,
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

#[derive(Debug, Clone, Template, TypedBuilder)]
#[template(path = "init/README.j2", escape = "md")]
pub struct InitReadmeTemplate<'a> {
    #[builder(setter(into))]
    repo_name: Cow<'a, str>,

    #[builder(setter(into))]
    registry: Cow<'a, str>,

    #[builder(setter(into))]
    image_name: Cow<'a, str>,
}

fn has_cosign_file() -> bool {
    trace!("has_cosign_file()");
    std::env::current_dir()
        .map(|p| p.join(COSIGN_PUB_PATH).exists())
        .unwrap_or(false)
}

#[must_use]
fn print_containerfile(containerfile: &str) -> String {
    trace!("print_containerfile({containerfile})");
    debug!("Loading containerfile contents for {containerfile}");

    let legacy_path = Path::new(CONFIG_PATH);
    let containerfiles_path = Path::new(CONTAINERFILES_PATH);

    let path = if containerfiles_path.exists() && containerfiles_path.is_dir() {
        containerfiles_path.join(format!("{containerfile}/{CONTAINER_FILE}"))
    } else {
        warn!("Use of {CONFIG_PATH} is deprecated for the containerfile module, please move your containerfile directories into {CONTAINERFILES_PATH}");
        legacy_path.join(format!("containerfiles/{containerfile}/{CONTAINER_FILE}"))
    };

    let file = fs::read_to_string(&path).unwrap_or_else(|e| {
        error!("Failed to read file {}: {e}", path.display());
        process::exit(1);
    });

    debug!("Containerfile contents {}:\n{file}", path.display());

    file
}

fn modules_exists() -> bool {
    let mod_path = Path::new("modules");
    mod_path.exists() && mod_path.is_dir()
}

fn files_dir_exists() -> bool {
    let path = Path::new(FILES_PATH);
    path.exists() && path.is_dir()
}

fn config_dir_exists() -> bool {
    let path = Path::new(CONFIG_PATH);
    let exists = path.exists() && path.is_dir();

    if exists {
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
