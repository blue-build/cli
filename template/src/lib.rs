use std::{borrow::Cow, fs, path::Path, process};

use blue_build_recipe::{MaybeVersion, Recipe};
use blue_build_utils::constants::{
    CONFIG_PATH, CONTAINER_FILE, CONTAINERFILES_PATH, COSIGN_PUB_PATH, FILES_PATH,
};
use bon::Builder;
use chrono::Utc;
use colored::control::ShouldColorize;
use log::{debug, error, trace, warn};
use oci_distribution::Reference;
use uuid::Uuid;

pub use askama::Template;

#[derive(Debug, Clone, Template, Builder)]
#[template(path = "Containerfile.j2", escape = "none", whitespace = "minimize")]
pub struct ContainerFileTemplate<'a> {
    #[builder(into)]
    recipe: &'a Recipe<'a>,
    recipe_path: &'a Path,

    #[builder(into)]
    build_id: Uuid,
    os_version: u64,
    registry: &'a str,
    build_scripts_image: &'a Reference,
    repo: &'a str,
    base_digest: &'a str,
    nushell_version: Option<&'a MaybeVersion>,

    #[builder(default)]
    build_features: &'a [String],
}

impl ContainerFileTemplate<'_> {
    const fn should_install_nu(&self) -> bool {
        match self.nushell_version {
            None | Some(MaybeVersion::Version(_)) => true,
            Some(MaybeVersion::None) => false,
        }
    }

    fn get_nu_version(&self) -> String {
        match self.nushell_version {
            Some(MaybeVersion::None) | None => "default".to_string(),
            Some(MaybeVersion::Version(version)) => version.to_string(),
        }
    }

    #[must_use]
    fn get_features(&self) -> String {
        self.build_features
            .iter()
            .map(|feat| feat.trim())
            .collect::<Vec<_>>()
            .join(",")
    }
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

#[derive(Debug, Clone, Template, Builder)]
#[template(path = "init/gitlab-ci.yml.j2", escape = "none")]
#[builder(on(Cow<'_, str>, into))]
pub struct GitlabCiTemplate<'a> {
    version: Cow<'a, str>,
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
        warn!(
            "Use of {CONFIG_PATH} is deprecated for the containerfile module, please move your containerfile directories into {CONTAINERFILES_PATH}"
        );
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
        warn!(
            "Use of the {CONFIG_PATH} directory is deprecated. Please move your non-recipe files into {FILES_PATH}"
        );
    }

    exists
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn should_color() -> bool {
    ShouldColorize::from_env().should_colorize()
}

mod filters {
    use askama::Values;

    #[allow(clippy::unnecessary_wraps)]
    pub fn replace<T>(input: T, _: &dyn Values, from: char, to: &str) -> askama::Result<String>
    where
        T: std::fmt::Display,
    {
        Ok(format!("{input}").replace(from, to))
    }
}
