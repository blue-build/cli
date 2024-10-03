use std::{
    fs,
    path::{Path, PathBuf},
};

use blue_build_process_management::{
    drivers::{
        opts::{
            BuildTagPushOpts, CheckKeyPairOpts, CompressionType, GenerateImageNameOpts,
            GenerateTagsOpts, SignVerifyOpts,
        },
        types::Platform,
        BuildDriver, CiDriver, Driver, DriverArgs, SigningDriver,
    },
    logging::{color_str, gen_random_ansi_color},
};
use blue_build_recipe::Recipe;
use blue_build_utils::{
    constants::{
        ARCHIVE_SUFFIX, BB_REGISTRY_NAMESPACE, BUILD_ID_LABEL, CONFIG_PATH, CONTAINER_FILE,
        GITIGNORE_PATH, LABELED_ERROR_MESSAGE, NO_LABEL_ERROR_MESSAGE, RECIPE_FILE, RECIPE_PATH,
    },
    cowstr,
    credentials::{Credentials, CredentialsArgs},
    string,
    traits::CowCollecter,
};
use bon::Builder;
use clap::Args;
use colored::Colorize;
use log::{info, trace, warn};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::commands::generate::GenerateCommand;

use super::BlueBuildCommand;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Args, Builder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    #[cfg(feature = "multi-recipe")]
    #[builder(into)]
    recipe: Option<Vec<PathBuf>>,

    /// The recipe file to build an image
    #[arg()]
    #[cfg(not(feature = "multi-recipe"))]
    #[builder(into)]
    recipe: Option<PathBuf>,

    /// Push the image with all the tags.
    ///
    /// Requires `--registry`,
    /// `--username`, and `--password` if not
    /// building in CI.
    #[arg(short, long)]
    #[builder(default)]
    push: bool,

    /// Build for a specific platform.
    ///
    /// NOTE: Building for a different architecture
    /// than your hardware will require installing
    /// qemu. Build times will be much greater when
    /// building for a non-native architecture.
    #[arg(long, default_value = "native")]
    #[builder(default)]
    platform: Platform,

    /// The compression format the images
    /// will be pushed in.
    #[arg(short, long, default_value_t = CompressionType::Gzip)]
    #[builder(default)]
    compression_format: CompressionType,

    /// Enable retrying to push the image.
    #[arg(short, long)]
    #[builder(default)]
    retry_push: bool,

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
    #[builder(into)]
    archive: Option<PathBuf>,

    /// The url path to your base
    /// project images.
    #[arg(long, env = BB_REGISTRY_NAMESPACE, visible_alias("registry-path"))]
    #[builder(into)]
    registry_namespace: Option<String>,

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
    credentials: CredentialsArgs,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for BuildCommand {
    /// Runs the command and returns a result.
    fn try_run(&mut self) -> Result<()> {
        trace!("BuildCommand::try_run()");

        Driver::init(self.drivers);

        Credentials::init(self.credentials.clone());

        self.update_gitignore()?;

        if self.push && self.archive.is_some() {
            bail!("You cannot use '--archive' and '--push' at the same time");
        }

        if self.push {
            blue_build_utils::check_command_exists("cosign")?;
            Driver::check_signing_files(&CheckKeyPairOpts::builder().dir(Path::new(".")).build())?;
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
                    .output(if recipe_paths.len() > 1 {
                        blue_build_utils::generate_containerfile_path(recipe)?
                    } else {
                        PathBuf::from(CONTAINER_FILE)
                    })
                    .platform(self.platform)
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
                .output(CONTAINER_FILE)
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

        trace!("BuildCommand::build_image()");

        let images = recipe_paths
            .par_iter()
            .try_fold(Vec::new, |mut images, recipe_path| -> Result<Vec<String>> {
                let containerfile = if recipe_paths.len() > 1 {
                    blue_build_utils::generate_containerfile_path(recipe_path)?
                } else {
                    PathBuf::from(CONTAINER_FILE)
                };
                images.extend(self.build(recipe_path, &containerfile)?);
                Ok(images)
            })
            .try_reduce(Vec::new, |mut init, image_names| {
                let color = gen_random_ansi_color();
                init.extend(image_names.iter().map(|image| color_str(image, color)));
                Ok(init)
            })?;

        info!(
            "Finished building:\n{}",
            images
                .iter()
                .map(|image| format!("\t- {image}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        Ok(())
    }

    #[cfg(not(feature = "multi-recipe"))]
    fn start(&self, recipe_path: &Path) -> Result<()> {
        trace!("BuildCommand::start()");

        let images = self.build(recipe_path, Path::new(CONTAINER_FILE))?;
        let color = gen_random_ansi_color();

        info!(
            "Finished building:\n{}",
            images
                .iter()
                .map(|image| format!("\t- {}", color_str(image, color)))
                .collect::<Vec<_>>()
                .join("\n")
        );
        Ok(())
    }

    fn build(&self, recipe_path: &Path, containerfile: &Path) -> Result<Vec<String>> {
        let recipe = Recipe::parse(recipe_path)?;
        let tags = Driver::generate_tags(
            &GenerateTagsOpts::builder()
                .oci_ref(&recipe.base_image_ref()?)
                .maybe_alt_tags(recipe.alt_tags.as_ref().map(CowCollecter::collect_cow_vec))
                .platform(self.platform)
                .build(),
        )?;
        let image_name = self.image_name(&recipe)?;

        let opts = if let Some(archive_dir) = self.archive.as_ref() {
            BuildTagPushOpts::builder()
                .containerfile(containerfile)
                .platform(self.platform)
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
                .containerfile(containerfile)
                .platform(self.platform)
                .tags(tags.collect_cow_vec())
                .push(self.push)
                .retry_push(self.retry_push)
                .retry_count(self.retry_count)
                .compression(self.compression_format)
                .squash(self.squash)
                .build()
        };

        let images = Driver::build_tag_push(&opts)?;

        if self.push && !self.no_sign {
            Driver::sign_and_verify(
                &SignVerifyOpts::builder()
                    .image(&image_name)
                    .retry_push(self.retry_push)
                    .retry_count(self.retry_count)
                    .maybe_tag(tags.first())
                    .platform(self.platform)
                    .build(),
            )?;
        }

        Ok(images)
    }

    fn image_name(&self, recipe: &Recipe) -> Result<String> {
        let image_name = Driver::generate_image_name(
            GenerateImageNameOpts::builder()
                .name(recipe.name.trim())
                .maybe_registry(self.credentials.registry.as_ref().map(|r| cowstr!(r)))
                .maybe_registry_namespace(self.registry_namespace.as_ref().map(|r| cowstr!(r)))
                .build(),
        )?;

        let image_name = if image_name.registry().is_empty() {
            string!(image_name.repository())
        } else if image_name.registry() == "" {
            image_name.repository().to_string()
        } else {
            format!("{}/{}", image_name.registry(), image_name.repository())
        };

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
