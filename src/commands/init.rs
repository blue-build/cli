use std::{
    env,
    fmt::{Display, Write as FmtWrite},
    fs::{self, OpenOptions},
    io::{BufWriter, Write as IoWrite},
    path::PathBuf,
    str::FromStr,
};

use blue_build_process_management::drivers::{
    CiDriver, Driver, DriverArgs, GitlabDriver, SigningDriver, opts::GenerateKeyPairOpts,
};
use blue_build_template::{GitlabCiTemplate, InitReadmeTemplate, Template};
use blue_build_utils::constants::{COSIGN_PUB_PATH, RECIPE_FILE, RECIPE_PATH, TEMPLATE_REPO_URL};
use bon::Builder;
use clap::{Args, ValueEnum, crate_version};
use comlexr::cmd;
use log::{debug, info, trace};
use miette::{Context, IntoDiagnostic, Report, Result, bail, miette};
use requestty::{Answer, Answers, OnEsc, questions};
use semver::Version;

use crate::commands::BlueBuildCommand;

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum CiProvider {
    #[default]
    Github,
    Gitlab,
    None,
}

impl CiProvider {
    fn default_ci_file_path(self) -> std::path::PathBuf {
        match self {
            Self::Gitlab => GitlabDriver::default_ci_file_path(),
            Self::None | Self::Github => unimplemented!(),
        }
    }

    fn render_file(self) -> Result<String> {
        match self {
            Self::Gitlab => GitlabCiTemplate::builder()
                .version({
                    let version = crate_version!();
                    let version: Version = version.parse().into_diagnostic()?;

                    format!("v{}.{}", version.major, version.minor)
                })
                .build()
                .render()
                .into_diagnostic(),
            Self::None | Self::Github => unimplemented!(),
        }
    }
}

impl TryFrom<&str> for CiProvider {
    type Error = Report;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            "Gitlab" => Self::Gitlab,
            "Github" => Self::Github,
            "None" => Self::None,
            _ => bail!("Unable to parse for CiProvider"),
        })
    }
}

impl TryFrom<&String> for CiProvider {
    type Error = Report;

    fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for CiProvider {
    type Err = Report;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl Display for CiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Github => "Github",
                Self::Gitlab => "Gitlab",
                Self::None => "None",
            }
        )
    }
}

#[derive(Debug, Clone, Default, Args, Builder)]
#[builder(on(String, into))]
pub struct NewInitCommon {
    /// The name of the image for the recipe.
    #[arg(long)]
    image_name: Option<String>,

    /// The name of the org where your repo will be located.
    /// This could end up being your username.
    #[arg(long)]
    org_name: Option<String>,

    /// Optional description for the GitHub repository.
    #[arg(long)]
    description: Option<String>,

    /// The registry to store the image.
    #[arg(long)]
    registry: Option<String>,

    /// The CI provider that will be building the image.
    ///
    /// GitHub Actions and Gitlab CI are currently the
    /// officially supported CI providers.
    #[arg(long, short)]
    ci_provider: Option<CiProvider>,

    /// Disable setting up git.
    #[arg(long)]
    no_git: bool,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

#[derive(Debug, Clone, Args, Builder)]
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

#[derive(Debug, Clone, Args, Builder)]
pub struct InitCommand {
    #[clap(skip)]
    #[builder(into)]
    dir: Option<PathBuf>,

    #[clap(flatten)]
    common: NewInitCommon,
}

impl BlueBuildCommand for InitCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::init(self.common.drivers);

        let base_dir = self
            .dir
            .get_or_insert(env::current_dir().into_diagnostic()?);

        if base_dir.exists() && fs::read_dir(base_dir).is_ok_and(|dir| dir.count() != 0) {
            bail!("Must be in an empty directory!");
        }

        self.start(&self.questions()?)
    }
}

macro_rules! when {
    ($check:expr) => {
        |_answers: &::requestty::Answers| $check
    };
}

impl InitCommand {
    const CI_PROVIDER: &str = "ci_provider";
    const REGISTRY: &str = "registry";
    const IMAGE_NAME: &str = "image_name";
    const ORG_NAME: &str = "org_name";
    const DESCRIPTION: &str = "description";

    fn questions(&self) -> Result<Answers> {
        let questions = questions![
            Input {
                name: Self::IMAGE_NAME,
                message: "What would you like to name your image?",
                when: when!(self.common.image_name.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Input {
                name: Self::REGISTRY,
                message: "What is the registry for the image? (e.g. ghcr.io or registry.gitlab.com)",
                when: when!(self.common.registry.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Input {
                name: Self::ORG_NAME,
                message: "What is the name of your org/username?",
                when: when!(self.common.org_name.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Input {
                name: Self::DESCRIPTION,
                message: "Write a short description of your image:",
                when: when!(self.common.description.is_none()),
                on_esc: OnEsc::Terminate,
            },
            Select {
                name: Self::CI_PROVIDER,
                message: "Are you building on Github or Gitlab?",
                when: when!(!self.common.no_git && self.common.ci_provider.is_none()),
                on_esc: OnEsc::Terminate,
                choices: vec!["Github", "Gitlab", "None"],
            }
        ];

        requestty::prompt(questions).into_diagnostic()
    }

    fn start(&self, answers: &Answers) -> Result<()> {
        self.clone_repository()?;
        self.remove_git_directory()?;
        self.template_readme(answers)?;
        self.template_ci_file(answers)?;
        self.update_recipe_file(answers)?;
        self.generate_signing_files()?;

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

        let mut command = cmd!(cd dir; "git", "commit", "-a", "-m", "chore: Initial Commit");
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

        let mut command = cmd!(cd dir; "git", "add", ".");
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

    fn template_readme(&self, answers: &Answers) -> Result<()> {
        trace!("template_readme()");

        let readme_path = self.dir.as_ref().unwrap().join("README.md");

        let readme = InitReadmeTemplate::builder()
            .repo_name(
                self.common
                    .org_name
                    .as_deref()
                    .or_else(|| answers.get(Self::ORG_NAME).and_then(Answer::as_string))
                    .ok_or_else(|| miette!("Failed to get organization name"))?,
            )
            .image_name(
                self.common
                    .image_name
                    .as_deref()
                    .or_else(|| answers.get(Self::IMAGE_NAME).and_then(Answer::as_string))
                    .ok_or_else(|| miette!("Failed to get image name"))?,
            )
            .registry(
                self.common
                    .registry
                    .as_deref()
                    .or_else(|| answers.get(Self::REGISTRY).and_then(Answer::as_string))
                    .ok_or_else(|| miette!("Failed to get registry"))?,
            )
            .build();

        debug!("Templating README");
        let readme = readme.render().into_diagnostic()?;

        debug!("Writing README to {}", readme_path.display());
        fs::write(readme_path, readme).into_diagnostic()
    }

    fn template_ci_file(&self, answers: &Answers) -> Result<()> {
        trace!("template_ci_file()");

        let ci_provider = self
            .common
            .ci_provider
            .ok_or("CLI Arg not set")
            .or_else(|e| {
                answers
                    .get(Self::CI_PROVIDER)
                    .and_then(Answer::as_list_item)
                    .map(|li| &li.text)
                    .ok_or_else(|| miette!("Failed to get CI Provider answer:\n{e}"))
                    .and_then(CiProvider::try_from)
            })?;

        if matches!(ci_provider, CiProvider::Github) {
            fs::remove_file(self.dir.as_ref().unwrap().join(".github/CODEOWNERS"))
                .into_diagnostic()?;
            return Ok(());
        }

        fs::remove_dir_all(self.dir.as_ref().unwrap().join(".github")).into_diagnostic()?;

        // Never run for None
        if matches!(ci_provider, CiProvider::None) {
            return Ok(());
        }

        let ci_file_path = self
            .dir
            .as_ref()
            .unwrap()
            .join(ci_provider.default_ci_file_path());
        let parent_path = ci_file_path
            .parent()
            .ok_or_else(|| miette!("Couldn't get parent directory from {ci_file_path:?}"))?;
        fs::create_dir_all(parent_path)
            .into_diagnostic()
            .with_context(|| format!("Couldn't create directory path {}", parent_path.display()))?;

        let file = &mut BufWriter::new(
            OpenOptions::new()
                .truncate(true)
                .create(true)
                .write(true)
                .open(&ci_file_path)
                .into_diagnostic()
                .with_context(|| format!("Failed to open file at {}", ci_file_path.display()))?,
        );

        let template = ci_provider.render_file()?;

        writeln!(file, "{template}")
            .into_diagnostic()
            .with_context(|| format!("Failed to write CI file {}", ci_file_path.display()))
    }

    fn update_recipe_file(&self, answers: &Answers) -> Result<()> {
        trace!("update_recipe_file()");

        let recipe_path = self
            .dir
            .as_ref()
            .unwrap()
            .join(RECIPE_PATH)
            .join(RECIPE_FILE);

        debug!("Reading {}", recipe_path.display());
        let file = fs::read_to_string(&recipe_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to read {}", recipe_path.display()))?;

        let description = self
            .common
            .description
            .as_deref()
            .ok_or("Description arg not set")
            .or_else(|e| {
                answers
                    .get(Self::DESCRIPTION)
                    .and_then(Answer::as_string)
                    .ok_or_else(|| miette!("Failed to get description:\n{e}"))
            })?;
        let name = self
            .common
            .image_name
            .as_deref()
            .ok_or("Description arg not set")
            .or_else(|e| {
                answers
                    .get(Self::IMAGE_NAME)
                    .and_then(Answer::as_string)
                    .ok_or_else(|| miette!("Failed to get description:\n{e}"))
            })?;

        let mut new_file_str = String::with_capacity(file.capacity());

        for line in file.lines() {
            if line.starts_with("description:") {
                writeln!(&mut new_file_str, "description: {description}").into_diagnostic()?;
            } else if line.starts_with("name: ") {
                writeln!(&mut new_file_str, "name: {name}").into_diagnostic()?;
            } else {
                writeln!(&mut new_file_str, "{line}").into_diagnostic()?;
            }
        }

        let file = &mut BufWriter::new(
            OpenOptions::new()
                .truncate(true)
                .write(true)
                .open(&recipe_path)
                .into_diagnostic()
                .with_context(|| format!("Failed to open {}", recipe_path.display()))?,
        );
        write!(file, "{new_file_str}")
            .into_diagnostic()
            .with_context(|| format!("Failed to write to file {}", recipe_path.display()))
    }

    fn generate_signing_files(&self) -> Result<()> {
        trace!("generate_signing_files()");

        debug!("Removing old cosign files {COSIGN_PUB_PATH}");
        fs::remove_file(self.dir.as_ref().unwrap().join(COSIGN_PUB_PATH))
            .into_diagnostic()
            .with_context(|| format!("Failed to delete old public file {COSIGN_PUB_PATH}"))?;

        Driver::generate_key_pair(
            GenerateKeyPairOpts::builder()
                .maybe_dir(self.dir.as_deref())
                .build(),
        )
    }
}
