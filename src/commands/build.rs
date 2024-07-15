use std::{
    env, fs,
    path::{Path, PathBuf},
};

use blue_build_recipe::Recipe;
use blue_build_utils::{
    constants::{
        ARCHIVE_SUFFIX, BB_PASSWORD, BB_REGISTRY, BB_REGISTRY_NAMESPACE, BB_USERNAME,
        BUILD_ID_LABEL, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_REGISTRY, CONFIG_PATH,
        CONTAINER_FILE, GITHUB_REPOSITORY_OWNER, GITIGNORE_PATH, LABELED_ERROR_MESSAGE,
        NO_LABEL_ERROR_MESSAGE, RECIPE_FILE, RECIPE_PATH,
    },
    generate_containerfile_path,
};
use clap::Args;
use colored::Colorize;
use log::{debug, info, trace, warn};
use miette::{bail, Context, IntoDiagnostic, Result};
use typed_builder::TypedBuilder;

use crate::{
    commands::generate::GenerateCommand,
    drivers::{
        opts::{BuildTagPushOpts, CompressionType},
        BuildDriver, Driver, SigningDriver,
    },
};

use super::{BlueBuildCommand, DriverArgs};

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    #[cfg(feature = "multi-recipe")]
    #[builder(default, setter(into, strip_option))]
    recipe: Option<Vec<PathBuf>>,

    /// The recipe file to build an image
    #[arg()]
    #[cfg(not(feature = "multi-recipe"))]
    #[builder(default, setter(into, strip_option))]
    recipe: Option<PathBuf>,

    /// Push the image with all the tags.
    ///
    /// Requires `--registry`,
    /// `--username`, and `--password` if not
    /// building in CI.
    #[arg(short, long)]
    #[builder(default)]
    push: bool,

    /// The compression format the images
    /// will be pushed in.
    #[arg(short, long, default_value_t = CompressionType::Gzip)]
    #[builder(default)]
    compression_format: CompressionType,

    /// Block `bluebuild` from retrying to push the image.
    #[arg(short, long, default_value_t = true)]
    #[builder(default)]
    no_retry_push: bool,

    /// The number of times to retry pushing the image.
    #[arg(long, default_value_t = 1)]
    #[builder(default)]
    retry_count: u8,

    /// Allow `bluebuild` to overwrite an existing
    /// Containerfile without confirmation.
    ///
    /// This is not needed if the Containerfile is in
    /// .gitignore or has already been built by `bluebuild`.
    #[arg(short, long)]
    #[builder(default)]
    force: bool,

    /// Archives the built image into a tarfile
    /// in the specified directory.
    #[arg(short, long)]
    #[builder(default, setter(into, strip_option))]
    archive: Option<PathBuf>,

    /// The registry's domain name.
    #[arg(long, env = BB_REGISTRY)]
    #[builder(default, setter(into, strip_option))]
    registry: Option<String>,

    /// The url path to your base
    /// project images.
    #[arg(long, env = BB_REGISTRY_NAMESPACE)]
    #[builder(default, setter(into, strip_option))]
    #[arg(visible_alias("registry-path"))]
    registry_namespace: Option<String>,

    /// The username to login to the
    /// container registry.
    #[arg(short = 'U', long, env = BB_USERNAME, hide_env_values = true)]
    #[builder(default, setter(into, strip_option))]
    username: Option<String>,

    /// The password to login to the
    /// container registry.
    #[arg(short = 'P', long, env = BB_PASSWORD, hide_env_values = true)]
    #[builder(default, setter(into, strip_option))]
    password: Option<String>,

    /// Do not sign the image on push.
    #[arg(long)]
    #[builder(default)]
    no_sign: bool,

    /// Runs all instructions inside one layer of the final image.
    ///
    /// WARN: This doesn't work with the
    /// docker driver as it has been deprecated.
    ///
    /// NOTE: Squash has a performance benefit for
    /// podman and buildah when running inside a container.
    #[arg(short, long)]
    #[builder(default)]
    squash: bool,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for BuildCommand {
    /// Runs the command and returns a result.
    fn try_run(&mut self) -> Result<()> {
        trace!("BuildCommand::try_run()");

        Driver::builder()
            .username(self.username.as_ref())
            .password(self.password.as_ref())
            .registry(self.registry.as_ref())
            .build_driver(self.drivers.build_driver)
            .inspect_driver(self.drivers.inspect_driver)
            .build()
            .init();

        self.update_gitignore()?;

        if self.push && self.archive.is_some() {
            bail!("You cannot use '--archive' and '--push' at the same time");
        }

        if self.push {
            blue_build_utils::check_command_exists("cosign")?;
            Driver::check_signing_files()?;
            Driver::login()?;
            Driver::signing_login()?;
        }

        #[cfg(feature = "multi-recipe")]
        {
            use rayon::prelude::*;
            let recipe_paths = self.recipe.clone().map_or_else(|| {
                let legacy_path = Path::new(CONFIG_PATH);
                let recipe_path = Path::new(RECIPE_PATH);
                if recipe_path.exists() && recipe_path.is_dir() {
                    vec![recipe_path.join(RECIPE_FILE)]
                } else {
                    warn!("Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}");
                    vec![legacy_path.join(RECIPE_FILE)]
                }
            },
            |recipes| {
                let mut same = std::collections::HashSet::new();

                recipes.into_iter().filter(|recipe| same.insert(recipe.clone())).collect()
            });

            recipe_paths.par_iter().try_for_each(|recipe| {
                GenerateCommand::builder()
                    .output(generate_containerfile_path(recipe)?)
                    .recipe(recipe)
                    .drivers(self.drivers)
                    .build()
                    .try_run()
            })?;

            self.start(&recipe_paths)
        }

        #[cfg(not(feature = "multi-recipe"))]
        {
            let recipe_path = self.recipe.clone().unwrap_or_else(|| {
                let legacy_path = Path::new(CONFIG_PATH);
                let recipe_path = Path::new(RECIPE_PATH);
                if recipe_path.exists() && recipe_path.is_dir() {
                    recipe_path.join(RECIPE_FILE)
                } else {
                    warn!("Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}");
                    legacy_path.join(RECIPE_FILE)
                }
            });

            GenerateCommand::builder()
                .output(generate_containerfile_path(&recipe_path)?)
                .recipe(&recipe_path)
                .drivers(self.drivers)
                .build()
                .try_run()?;

            self.start(&recipe_path)
        }
    }
}

impl BuildCommand {
    #[cfg(feature = "multi-recipe")]
    fn start(&self, recipe_paths: &[PathBuf]) -> Result<()> {
        use rayon::prelude::*;

        use crate::drivers::BuildDriver;
        trace!("BuildCommand::build_image()");

        recipe_paths
            .par_iter()
            .try_for_each(|recipe_path| -> Result<()> {
                let recipe = Recipe::parse(recipe_path)?;
                let os_version = Driver::get_os_version(&recipe)?;
                let containerfile = generate_containerfile_path(recipe_path)?;
                let tags = recipe.generate_tags(os_version);
                let image_name = self.generate_full_image_name(&recipe)?;

                let opts = if let Some(archive_dir) = self.archive.as_ref() {
                    BuildTagPushOpts::builder()
                        .containerfile(&containerfile)
                        .archive_path(format!(
                            "{}/{}.{ARCHIVE_SUFFIX}",
                            archive_dir.to_string_lossy().trim_end_matches('/'),
                            recipe.name.to_lowercase().replace('/', "_"),
                        ))
                        .squash(self.squash)
                        .build()
                } else {
                    BuildTagPushOpts::builder()
                        .image(&image_name)
                        .containerfile(&containerfile)
                        .tags(tags.iter().map(String::as_str).collect::<Vec<_>>())
                        .push(self.push)
                        .no_retry_push(self.no_retry_push)
                        .retry_count(self.retry_count)
                        .compression(self.compression_format)
                        .squash(self.squash)
                        .build()
                };

                Driver::build_tag_push(&opts)?;

                if self.push && !self.no_sign {
                    Driver::sign_images(&image_name, tags.first().map(String::as_str))?;
                }

                Ok(())
            })?;

        info!("Build complete!");
        Ok(())
    }

    #[cfg(not(feature = "multi-recipe"))]
    fn start(&self, recipe_path: &Path) -> Result<()> {
        trace!("BuildCommand::start()");

        let recipe = Recipe::parse(recipe_path)?;
        let os_version = Driver::get_os_version(&recipe)?;
        let containerfile = generate_containerfile_path(recipe_path)?;
        let tags = recipe.generate_tags(os_version);
        let image_name = self.generate_full_image_name(&recipe)?;

        let opts = if let Some(archive_dir) = self.archive.as_ref() {
            BuildTagPushOpts::builder()
                .containerfile(&containerfile)
                .archive_path(format!(
                    "{}/{}.{ARCHIVE_SUFFIX}",
                    archive_dir.to_string_lossy().trim_end_matches('/'),
                    recipe.name.to_lowercase().replace('/', "_"),
                ))
                .squash(self.squash)
                .build()
        } else {
            BuildTagPushOpts::builder()
                .image(&image_name)
                .containerfile(&containerfile)
                .tags(tags.iter().map(String::as_str).collect::<Vec<_>>())
                .push(self.push)
                .no_retry_push(self.no_retry_push)
                .retry_count(self.retry_count)
                .compression(self.compression_format)
                .squash(self.squash)
                .build()
        };

        Driver::build_tag_push(&opts)?;

        if self.push && !self.no_sign {
            Driver::sign_images(&image_name, tags.first().map(String::as_str))?;
        }

        info!("Build complete!");
        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if the image name cannot be generated.
    pub fn generate_full_image_name(&self, recipe: &Recipe) -> Result<String> {
        trace!("BuildCommand::generate_full_image_name({recipe:#?})");
        info!("Generating full image name");

        let image_name = match (
            env::var(CI_REGISTRY).ok().map(|s| s.to_lowercase()),
            env::var(CI_PROJECT_NAMESPACE)
                .ok()
                .map(|s| s.to_lowercase()),
            env::var(CI_PROJECT_NAME).ok().map(|s| s.to_lowercase()),
            env::var(GITHUB_REPOSITORY_OWNER)
                .ok()
                .map(|s| s.to_lowercase()),
            self.registry.as_ref().map(|s| s.to_lowercase()),
            self.registry_namespace.as_ref().map(|s| s.to_lowercase()),
        ) {
            (_, _, _, _, Some(registry), Some(registry_path)) => {
                trace!("registry={registry}, registry_path={registry_path}");
                format!(
                    "{}/{}/{}",
                    registry.trim().trim_matches('/'),
                    registry_path.trim().trim_matches('/'),
                    recipe.name.trim(),
                )
            }
            (
                Some(ci_registry),
                Some(ci_project_namespace),
                Some(ci_project_name),
                None,
                None,
                None,
            ) => {
                trace!("CI_REGISTRY={ci_registry}, CI_PROJECT_NAMESPACE={ci_project_namespace}, CI_PROJECT_NAME={ci_project_name}");
                warn!("Generating Gitlab Registry image");
                format!(
                    "{ci_registry}/{ci_project_namespace}/{ci_project_name}/{}",
                    recipe.name.trim().to_lowercase()
                )
            }
            (None, None, None, Some(github_repository_owner), None, None) => {
                trace!("GITHUB_REPOSITORY_OWNER={github_repository_owner}");
                warn!("Generating Github Registry image");
                format!("ghcr.io/{github_repository_owner}/{}", &recipe.name)
            }
            _ => {
                trace!("Nothing to indicate an image name with a registry");
                if self.push {
                    bail!("Need '--registry' and '--registry-namespace' in order to push image");
                }
                recipe.name.trim().to_lowercase()
            }
        };

        debug!("Using image name '{image_name}'");

        Ok(image_name)
    }

    fn update_gitignore(&self) -> Result<()> {
        // Check if the Containerfile exists
        //   - If doesn't => *Build*
        //   - If it does:
        //     - check entry in .gitignore
        //       -> If it is => *Build*
        //       -> If isn't:
        //         - check if it has the BlueBuild tag (LABEL)
        //           -> If it does => *Ask* to add to .gitignore and remove from git
        //           -> If it doesn't => *Ask* to continue and override the file

        let container_file_path = Path::new(CONTAINER_FILE);
        let label = format!("LABEL {BUILD_ID_LABEL}");

        if !self.force && container_file_path.exists() {
            let to_ignore_lines = [format!("/{CONTAINER_FILE}"), format!("/{CONTAINER_FILE}.*")];
            let gitignore = fs::read_to_string(GITIGNORE_PATH)
                .into_diagnostic()
                .with_context(|| format!("Failed to read {GITIGNORE_PATH}"))?;

            let mut edited_gitignore = gitignore.clone();

            to_ignore_lines
                .iter()
                .filter(|to_ignore| {
                    !gitignore
                        .lines()
                        .any(|line| line.trim() == to_ignore.trim())
                })
                .try_for_each(|to_ignore| -> Result<()> {
                    let containerfile = fs::read_to_string(container_file_path)
                        .into_diagnostic()
                        .with_context(|| {
                        format!("Failed to read {}", container_file_path.display())
                    })?;

                    let has_label = containerfile
                        .lines()
                        .any(|line| line.to_string().trim().starts_with(&label));

                    let question = requestty::Question::confirm("build")
                        .message(
                            if has_label {
                                LABELED_ERROR_MESSAGE
                            } else {
                                NO_LABEL_ERROR_MESSAGE
                            }
                            .bright_yellow()
                            .to_string(),
                        )
                        .default(true)
                        .build();

                    if let Ok(answer) = requestty::prompt_one(question) {
                        if answer.as_bool().unwrap_or(false) {
                            if !edited_gitignore.ends_with('\n') {
                                edited_gitignore.push('\n');
                            }

                            edited_gitignore.push_str(to_ignore);
                            edited_gitignore.push('\n');
                        }
                    }
                    Ok(())
                })?;

            if edited_gitignore != gitignore {
                fs::write(GITIGNORE_PATH, edited_gitignore.as_str()).into_diagnostic()?;
            }
        }

        Ok(())
    }
}
