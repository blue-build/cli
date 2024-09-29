use std::{borrow::Cow, fs, path::Path, process};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::{
    CONFIG_PATH, CONTAINERFILES_PATH, CONTAINER_FILE, COSIGN_PUB_PATH, FILES_PATH,
};
use bon::Builder;
use colored::control::ShouldColorize;
use log::{debug, error, trace, warn};
use uuid::Uuid;

pub use rinja::Template;

#[derive(Debug, Clone, Template, Builder)]
#[template(path = "Containerfile.j2", escape = "none", whitespace = "minimize")]
#[builder(on(Cow<'_, str>, into))]
pub struct ContainerFileTemplate<'a> {
    #[builder(into)]
    recipe: &'a Recipe<'a>,

    #[builder(into)]
    recipe_path: Cow<'a, Path>,

    #[builder(into)]
    build_id: Uuid,
    os_version: u64,
    registry: Cow<'a, str>,
    exports_tag: Cow<'a, str>,
    repo: Cow<'a, str>,
}

#[derive(Debug, Clone, Template, Builder)]
#[template(path = "github_issue.j2", escape = "md")]
#[builder(on(Cow<'_, str>, into))]
pub struct GithubIssueTemplate<'a> {
    bb_version: Cow<'a, str>,
    build_rust_channel: Cow<'a, str>,
    build_time: Cow<'a, str>,
    git_commit_hash: Cow<'a, str>,
    os_name: Cow<'a, str>,
    os_version: Cow<'a, str>,
    pkg_branch_tag: Cow<'a, str>,
    recipe: Cow<'a, str>,
    rust_channel: Cow<'a, str>,
    rust_version: Cow<'a, str>,
    shell_name: Cow<'a, str>,
    shell_version: Cow<'a, str>,
    terminal_name: Cow<'a, str>,
    terminal_version: Cow<'a, str>,
}

#[derive(Debug, Clone, Template, Builder)]
#[template(path = "init/README.j2", escape = "md")]
#[builder(on(Cow<'_, str>, into))]
pub struct InitReadmeTemplate<'a> {
    repo_name: Cow<'a, str>,
    registry: Cow<'a, str>,
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

fn should_color() -> bool {
    ShouldColorize::from_env().should_colorize()
}

mod filters {
    #[allow(clippy::unnecessary_wraps)]
    pub fn replace<T>(input: T, from: char, to: &str) -> rinja::Result<String>
    where
        T: std::fmt::Display,
    {
        Ok(format!("{input}").replace(from, to))
    }
}
