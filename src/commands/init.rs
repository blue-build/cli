use std::{env, fs, path::PathBuf, process::Command};

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
        let base_dir = self.dir.get_or_insert(env::current_dir()?);

        if base_dir.exists() && fs::read_dir(&base_dir).is_ok_and(|dir| dir.count() != 0) {
            bail!("Must be in an empty directory!");
        }

        // Clone the template repository
        self.clone_repository()?;

        Ok(())
    }
}

impl InitCommand {
    fn clone_repository(&self) -> Result<()> {
        let dir = self.dir.as_ref().unwrap();
        let dir_display = dir.display();
        trace!("clone_repository({dir_display})");

        trace!("git clone {TEMPLATE_REPO_URL} {dir_display}");
        let status = Command::new("git")
            .args(["clone", "-q", TEMPLATE_REPO_URL])
            .arg(dir)
            .status()
            .context("Failed to execute git clone")?;

        if !status.success() {
            bail!("Failed to clone template repo");
        }

        self.remove_git_directory()?;
        self.remove_codeowners_file()?;

        if !self.common.no_git {
            self.initialize_git()?;
        }

        info!("Created new BlueBuild project in {dir_display}");
        Ok(())
    }

    fn remove_git_directory(&self) -> Result<()> {
        let dir = self.dir.as_ref().unwrap();
        let git_path = dir.join(".git");

        if git_path.exists() {
            fs::remove_dir_all(&git_path).context("Failed to remove .git directory")?;
            debug!(".git directory removed.");
        }
        Ok(())
    }

    fn remove_codeowners_file(&self) -> Result<()> {
        let dir = self.dir.as_ref().unwrap();
        let codeowners_path = dir.join(".github/CODEOWNERS");

        if codeowners_path.exists() {
            fs::remove_file(codeowners_path).context("Failed to remove CODEOWNERS file")?;
            debug!("CODEOWNERS file removed.");
        }

        Ok(())
    }

    fn initialize_git(&self) -> Result<()> {
        let dir = self.dir.as_ref().unwrap();
        let status = Command::new("git")
            .args(["init", "-q", "-b=main"])
            .arg(dir)
            .status()
            .context("Failed to execute git init")?;

        if !status.success() {
            bail!("Error initializing git");
        }

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
