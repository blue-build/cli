use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::commands::BlueBuildCommand;

#[derive(Debug, Clone, Default, Args, TypedBuilder)]
pub struct NewInitCommon {
    #[arg(long)]
    #[builder(default)]
    no_git: bool,

    #[arg(long)]
    #[builder(default)]
    github_setup: bool,

    /// GitHub authentication token for setting up the repository
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    github_token: Option<String>,

    /// Name of the GitHub repository to create or fork
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    repo_name: Option<String>,

    /// Optional description for the GitHub repository
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    repo_description: Option<String>,

    /// Whether to use a template repository for creating the new repo
    #[arg(long)]
    #[builder(default)]
    use_template: bool,
}

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct InitCommand {
    /// The directory to extract the files into. Defaults to the current directory
    #[arg()]
    #[builder(setter(strip_option, into), default)]
    dir: Option<PathBuf>,

    #[clap(flatten)]
    #[builder(default)]
    common: NewInitCommon,
}

#[derive(Serialize, Debug, Clone, TypedBuilder)]
struct GitHubRepoRequest {
    #[builder(setter(into))]
    name: String,

    #[builder(setter(into))]
    description: String,

    #[builder(default)]
    private: bool,
}

#[derive(Deserialize, Debug, Clone, TypedBuilder)]
struct GitHubRepoResponse {
    html_url: String,
    clone_url: String,
}

impl BlueBuildCommand for InitCommand {
    fn try_run(&mut self) -> Result<()> {
        let base_dir = self.dir.clone().unwrap_or_else(|| PathBuf::from("./"));

        if self.common.github_setup {
            let token = self
                .common
                .github_token
                .as_ref()
                .ok_or_else(|| anyhow!("GitHub token is required for setup"))?;
            let repo_name = self
                .common
                .repo_name
                .as_ref()
                .ok_or_else(|| anyhow!("Repository name is required for GitHub setup"))?;

            let api_url = "https://api.github.com/user/repos"; // Direct repo creation URL

            let repo_request = GitHubRepoRequest::builder()
                .name(repo_name.clone())
                .description(
                    self.common
                        .repo_description
                        .as_ref()
                        .map_or_else(|| "This is my personal OS image.", |d| d),
                )
                .private(true)
                .build();

            let response: GitHubRepoResponse = ureq::post(api_url)
                .set("Authorization", &format!("token {token}"))
                .set("Accept", "application/vnd.github.v3+json")
                .send_json(ureq::json!(repo_request))?
                .into_json()?;

            println!("Repository created: {}", response.html_url);

            clone_repository(&response.clone_url, &base_dir)?;
        } else {
            let template_repo_url = "https://github.com/blue-build/template.git"; // Replace with your template repo URL

            // Clone the template repository
            clone_repository(template_repo_url, &base_dir)?;

            if self.common.no_git {
                // If no_git is true, remove the .git directory to disable git
                remove_git_directory(&base_dir)?;
            } else {
                // Remove any existing remotes if not using GitHub setup
                remove_git_remotes(&base_dir)?;
            }
        }

        Ok(())
    }
}

fn clone_repository(repo_url: &str, dir: &Path) -> Result<()> {
    Command::new("git")
        .args([
            "clone",
            repo_url,
            dir.to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid directory path"))?,
        ])
        .status()
        .context("Failed to execute git clone")?;

    println!("Repository cloned successfully into {}", dir.display());
    Ok(())
}

fn remove_git_directory(dir: &Path) -> Result<()> {
    let git_path = dir.join(".git");
    if git_path.exists() {
        fs::remove_dir_all(&git_path).context("Failed to remove .git directory")?;
        println!(".git directory removed for local only development.");
    }
    Ok(())
}

fn remove_git_remotes(dir: &Path) -> Result<()> {
    Command::new("git")
        .args([
            "-C",
            dir.to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid directory path"))?,
            "remote",
            "remove",
            "origin",
        ])
        .status()
        .context("Failed to remove git remote")?;

    println!("Git remote removed.");
    Ok(())
}

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct NewCommand {
    #[arg()]
    dir: PathBuf,

    #[clap(flatten)]
    common: NewInitCommon,
}

impl BlueBuildCommand for NewCommand {
    fn try_run(&mut self) -> Result<()> {
        InitCommand::builder()
            .dir(self.dir.clone())
            .common(self.common.clone())
            .build()
            .try_run()
    }
}
