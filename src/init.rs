use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args;
use typed_builder::TypedBuilder;

const GITLAB_CI_FILE: &'static str = include_str!("../templates/init/gitlab-ci.yml.tera");
const RECIPE_FILE: &'static str = include_str!("../templates/init/recipe.yml.tera");
const LICENSE_FILE: &'static str = include_str!("../LICENSE");

#[derive(Debug, Clone, Default, Args, TypedBuilder)]
pub struct NewInitCommon {
    #[builder(default)]
    no_git: bool,
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

impl InitCommand {
    pub fn run(&self) -> Result<()> {
        let base_dir = match self.dir.as_ref() {
            Some(dir) => dir,
            None => std::path::Path::new("./"),
        };

        self.initialize_directory(base_dir);
        Ok(())
    }

    fn initialize_directory(&self, base_dir: &Path) {
        let recipe_path = base_dir.join("recipe.yml");

        let gitlab_ci_path = base_dir.join(".gitlab-ci.yml");

        let readme_path = base_dir.join("README.md");

        let license_path = base_dir.join("LICENSE");

        let scripts_dir = base_dir.join("scripts/");

        let pre_scripts_dir = scripts_dir.join("pre/");

        let post_scripts_dir = scripts_dir.join("post/");
    }
}

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct NewCommand {
    #[arg()]
    dir: PathBuf,

    #[clap(flatten)]
    common: NewInitCommon,
}

impl NewCommand {
    pub fn run(&self) -> Result<()> {
        InitCommand::builder()
            .dir(self.dir.clone())
            .common(self.common.clone())
            .build()
            .run()?;

        Ok(())
    }
}
