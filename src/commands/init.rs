use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use blue_build_utils::constants::TEMPLATE_REPO_URL;
use clap::Args;
use log::{debug, info, trace};
use typed_builder::TypedBuilder;

use crate::commands::BlueBuildCommand;

#[derive(Debug, Clone, Default, Args, TypedBuilder)]
pub struct NewInitCommon {
    #[arg(long)]
    #[builder(default)]
    no_git: bool,

    /// Name of the GitHub repository to create
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    repo_name: Option<String>,

    /// Optional description for the GitHub repository
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    repo_description: Option<String>,
}

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct InitCommand {
    #[clap(skip)]
    #[builder(setter(into), default)]
    dir: Option<PathBuf>,

    #[clap(flatten)]
    #[builder(default)]
    common: NewInitCommon,
}

impl BlueBuildCommand for InitCommand {
    fn try_run(&mut self) -> Result<()> {
        let base_dir = self.dir.get_or_insert(PathBuf::from("./"));

        if base_dir.exists() && fs::read_dir(&base_dir).is_ok_and(|dir| dir.count() != 0) {
            bail!("Must be in an empty directory!");
        }

        // Clone the template repository
        Self::clone_repository(base_dir)?;

        if self.common.no_git {
            // If no_git is true, remove the .git directory to disable git
            Self::remove_git_directory(base_dir)?;
        } else {
            // Remove any existing remotes if not using GitHub setup
            Self::remove_git_remotes(base_dir)?;
        }

        Ok(())
    }
}

impl InitCommand {
    fn clone_repository(dir: &Path) -> Result<()> {
        let dir_display = dir.display();
        trace!("clone_repository({dir_display})");

        trace!("git clone {TEMPLATE_REPO_URL} {dir_display}");
        let status = Command::new("git")
            .args(["clone", TEMPLATE_REPO_URL])
            .arg(dir)
            .status()
            .context("Failed to execute git clone")?;

        if !status.success() {
            bail!("Failed to clone template repo");
        }

        info!("Repository cloned successfully into {dir_display}");
        Ok(())
    }

    fn remove_git_directory(dir: &Path) -> Result<()> {
        let git_path = dir.join(".git");
        if git_path.exists() {
            fs::remove_dir_all(&git_path).context("Failed to remove .git directory")?;
            debug!(".git directory removed for local only development.");
        }
        Ok(())
    }

    fn remove_git_remotes(dir: &Path) -> Result<()> {
        let status = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["remote", "remove", "origin"])
            .status()
            .context("Failed to remove git remote")?;

        if !status.success() {
            bail!("Couldn't remove origin");
        }

        debug!("Git remote removed.");
        Ok(())
    }
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
