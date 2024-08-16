use std::{env, fs, path::PathBuf};

use blue_build_template::{InitReadmeTemplate, Template};
use blue_build_utils::{cmd, constants::TEMPLATE_REPO_URL};
use clap::{Args, ValueEnum};
use log::{debug, info, trace};
use miette::{bail, Context, IntoDiagnostic, Result};
use requestty::{questions, Answers, OnEsc};
use typed_builder::TypedBuilder;

use crate::commands::BlueBuildCommand;

#[derive(Debug, Default, Clone, ValueEnum)]
pub enum CiProvider {
    #[default]
    Github,
    Gitlab,
    None,
}

#[derive(Debug, Clone, Default, Args, TypedBuilder)]
pub struct NewInitCommon {
    /// The name of the image for the recipe.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    image_name: Option<String>,

    /// The name of the org where your repo will be located.
    /// This could end up being your username.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    org_name: Option<String>,

    /// Optional description for the GitHub repository.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    description: Option<String>,

    /// The CI provider that will be building the image.
    ///
    /// GitHub Actions and Gitlab CI are currently the
    /// officially supported CI providers.
    #[arg(long, short)]
    #[builder(default)]
    ci_provider: Option<CiProvider>,

    /// Disable setting up git.
    #[arg(long)]
    #[builder(default)]
    no_git: bool,
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
        let base_dir = self
            .dir
            .get_or_insert(env::current_dir().into_diagnostic()?);

        if base_dir.exists() && fs::read_dir(base_dir).is_ok_and(|dir| dir.count() != 0) {
            bail!("Must be in an empty directory!");
        }

        self.start(&self.questions()?)
    }
}

macro_rules! impl_when {
    ($check:expr) => {
        |_answers: &::requestty::Answers| $check
    };
}

impl InitCommand {
    fn questions(&self) -> Result<Answers> {
        let questions = questions![
            Input {
                name: "image_name",
                message: "What would you like to name your image?",
                when: impl_when!(self.common.image_name.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Input {
                name: "org_name",
                message: "What is the name of your org/username?",
                when: impl_when!(self.common.org_name.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Input {
                name: "description",
                message: "Write a short description of your image:",
                when: impl_when!(self.common.description.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Select {
                name: "ci_provider",
                message: "Are you building on Github or Gitlab?",
                when: impl_when!(!self.common.no_git && self.common.ci_provider.is_none()),
                should_loop: true,
                on_esc: OnEsc::Terminate,
            }
        ];

        requestty::prompt(questions).into_diagnostic()
    }

    fn start(&self, _answers: &Answers) -> Result<()> {
        self.clone_repository()?;
        self.remove_git_directory()?;
        self.remove_codeowners_file()?;
        self.template_readme()?;
        self.set_ci_provider()?;

        if !self.common.no_git {
            self.initialize_git()?;
            self.add_files()?;
            self.initial_commit()?;
        }

        info!(
            "Created new BlueBuild project in {}",
            self.dir.as_ref().unwrap().display()
        );

        Ok(())
    }

    fn clone_repository(&self) -> Result<()> {
        let dir = self.dir.as_ref().unwrap();
        trace!("clone_repository()");

        let mut command = cmd!("git", "clone", "-q", TEMPLATE_REPO_URL, dir);
        trace!("{command:?}");

        let status = command
            .status()
            .into_diagnostic()
            .context("Failed to execute git clone")?;

        if !status.success() {
            bail!("Failed to clone template repo");
        }

        Ok(())
    }

    fn remove_git_directory(&self) -> Result<()> {
        trace!("remove_git_directory()");

        let dir = self.dir.as_ref().unwrap();
        let git_path = dir.join(".git");

        if git_path.exists() {
            fs::remove_dir_all(&git_path)
                .into_diagnostic()
                .context("Failed to remove .git directory")?;
            debug!(".git directory removed.");
        }
        Ok(())
    }

    fn remove_codeowners_file(&self) -> Result<()> {
        trace!("remove_codeowners_file()");

        let dir = self.dir.as_ref().unwrap();
        let codeowners_path = dir.join(".github/CODEOWNERS");

        if codeowners_path.exists() {
            fs::remove_file(codeowners_path)
                .into_diagnostic()
                .context("Failed to remove CODEOWNERS file")?;
            debug!("CODEOWNERS file removed.");
        }

        Ok(())
    }

    fn initialize_git(&self) -> Result<()> {
        trace!("initialize_git()");

        let dir = self.dir.as_ref().unwrap();

        let mut command = cmd!("git", "init", "-q", "-b", "main", dir);
        trace!("{command:?}");

        let status = command
            .status()
            .into_diagnostic()
            .context("Failed to execute git init")?;

        if !status.success() {
            bail!("Error initializing git");
        }

        debug!("Initialized git in {}", dir.display());

        Ok(())
    }

    fn initial_commit(&self) -> Result<()> {
        trace!("initial_commit()");

        let dir = self.dir.as_ref().unwrap();

        let mut command = cmd!(
            "git",
            "commit",
            "-a",
            "-m",
            "chore: Initial Commit",
            current_dir = dir,
        );
        trace!("{command:?}");

        let status = command
            .status()
            .into_diagnostic()
            .context("Failed to run git commit")?;

        if !status.success() {
            bail!("Failed to commit initial changes");
        }

        debug!("Created initial commit");

        Ok(())
    }

    fn add_files(&self) -> Result<()> {
        trace!("add_files()");

        let dir = self.dir.as_ref().unwrap();

        let mut command = cmd!("git", "add", ".", current_dir = dir,);
        trace!("{command:?}");

        let status = command
            .status()
            .into_diagnostic()
            .context("Failed to run git add")?;

        if !status.success() {
            bail!("Failed to add files to initial commit");
        }

        debug!("Added files for initial commit");

        Ok(())
    }

    fn template_readme(&self) -> Result<()> {
        trace!("template_readme()");

        let readme_path = self.dir.as_ref().unwrap().join("README.md");

        let readme = InitReadmeTemplate::builder()
            .repo_name(
                self.common
                    .org_name
                    .as_ref()
                    .map_or("image_repo", String::as_str),
            )
            .image_name(
                self.common
                    .image_name
                    .as_ref()
                    .map_or("template", String::as_str),
            )
            .registry("registry.example.io")
            .build();

        debug!("Templating README");
        let readme = readme.render().into_diagnostic()?;

        debug!("Writing README to {}", readme_path.display());
        fs::write(readme_path, readme).into_diagnostic()?;

        Ok(())
    }

    fn set_ci_provider(&self) -> Result<()> {
        todo!()
    }
}
