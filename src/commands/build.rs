use std::path::{Path, PathBuf};

use blue_build_process_management::{
    drivers::{
        BuildDriver, CiDriver, Driver, DriverArgs, SigningDriver,
        opts::{
            BuildTagPushOpts, CheckKeyPairOpts, CompressionType, GenerateImageNameOpts,
            GenerateTagsOpts, SignVerifyOpts,
        },
        types::{BuildDriverType, ImageRef, Platform, RunDriverType},
    },
    logging::{color_str, gen_random_ansi_color},
};
use blue_build_recipe::Recipe;
use blue_build_utils::{
    constants::{
        ARCHIVE_SUFFIX, BB_BUILD_ARCHIVE, BB_BUILD_NO_SIGN, BB_BUILD_PLATFORM, BB_BUILD_PUSH,
        BB_BUILD_RECHUNK, BB_BUILD_RECHUNK_CLEAR_PLAN, BB_BUILD_RETRY_COUNT, BB_BUILD_RETRY_PUSH,
        BB_BUILD_SQUASH, BB_CACHE_LAYERS, BB_REGISTRY_NAMESPACE, BB_SKIP_VALIDATION, BB_TEMPDIR,
        CONFIG_PATH, RECIPE_FILE, RECIPE_PATH,
    },
    credentials::{Credentials, CredentialsArgs},
    string,
};
use bon::Builder;
use clap::Args;
use log::{debug, info, trace, warn};
use miette::{IntoDiagnostic, Result, bail};
use oci_distribution::Reference;
use rayon::prelude::*;
use tempfile::TempDir;

use crate::commands::generate::{GenerateCommand, generate_default_labels};

use super::BlueBuildCommand;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Args, Builder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    #[builder(into)]
    recipe: Option<Vec<PathBuf>>,

    /// Push the image with all the tags.
    ///
    /// Requires `--registry`,
    /// `--username`, and `--password` if not
    /// building in CI.
    #[arg(short, long, group = "archive_push", env = BB_BUILD_PUSH)]
    #[builder(default)]
    push: bool,

    /// Build for a specific platform.
    ///
    /// NOTE: Building for a different architecture
    /// than your hardware will require installing
    /// qemu. Build times will be much greater when
    /// building for a non-native architecture.
    #[arg(long, env = BB_BUILD_PLATFORM)]
    platform: Option<Platform>,

    /// The compression format the images
    /// will be pushed in.
    #[arg(short, long, default_value_t = CompressionType::Gzip)]
    #[builder(default)]
    compression_format: CompressionType,

    /// Enable retrying to push the image.
    #[arg(short, long, env = BB_BUILD_RETRY_PUSH)]
    #[builder(default)]
    retry_push: bool,

    /// The number of times to retry pushing the image.
    #[arg(long, default_value_t = 1, env = BB_BUILD_RETRY_COUNT)]
    #[builder(default)]
    retry_count: u8,

    /// Archives the built image into a tarfile
    /// in the specified directory.
    #[arg(short, long, group = "archive_rechunk", group = "archive_push", env = BB_BUILD_ARCHIVE)]
    #[builder(into)]
    archive: Option<PathBuf>,

    /// The url path to your base
    /// project images.
    #[arg(long, env = BB_REGISTRY_NAMESPACE, visible_alias("registry-path"))]
    #[builder(into)]
    registry_namespace: Option<String>,

    /// Do not sign the image on push.
    #[arg(long, env = BB_BUILD_NO_SIGN)]
    #[builder(default)]
    no_sign: bool,

    /// Runs all instructions inside one layer of the final image.
    ///
    /// WARN: This doesn't work with the
    /// docker driver as it has been deprecated.
    ///
    /// NOTE: Squash has a performance benefit for
    /// podman and buildah when running inside a container.
    #[arg(short, long, env = BB_BUILD_SQUASH)]
    #[builder(default)]
    squash: bool,

    /// Performs rechunking on the image to allow for smaller images
    /// and smaller updates.
    ///
    /// WARN: This will increase the build-time
    /// and take up more space during build-time.
    ///
    /// NOTE: This must be run as root!
    #[arg(long, group = "archive_rechunk", env = BB_BUILD_RECHUNK)]
    #[builder(default)]
    rechunk: bool,

    /// Use a fresh rechunk plan, regardless of previous ref.
    ///
    /// NOTE: Only works with `--rechunk`.
    #[arg(long, env = BB_BUILD_RECHUNK_CLEAR_PLAN)]
    #[builder(default)]
    rechunk_clear_plan: bool,

    /// The location to temporarily store files
    /// while building. If unset, it will use `/tmp`.
    #[arg(long, env = BB_TEMPDIR)]
    tempdir: Option<PathBuf>,

    /// Automatically cache build layers to the registry.
    ///
    /// NOTE: Only works when using --push
    #[builder(default)]
    #[arg(long, env = BB_CACHE_LAYERS)]
    cache_layers: bool,

    /// Skips validation of the recipe file.
    #[arg(long, env = BB_SKIP_VALIDATION)]
    #[builder(default)]
    skip_validation: bool,

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

        Driver::init(if self.rechunk {
            DriverArgs::builder()
                .build_driver(BuildDriverType::Podman)
                .run_driver(RunDriverType::Podman)
                .maybe_boot_driver(self.drivers.boot_driver)
                .maybe_signing_driver(self.drivers.signing_driver)
                .build()
        } else {
            self.drivers
        });

        Credentials::init(self.credentials.clone());

        if self.push && self.archive.is_some() {
            bail!("You cannot use '--archive' and '--push' at the same time");
        }

        if self.push {
            blue_build_utils::check_command_exists("cosign")?;
            Driver::check_signing_files(CheckKeyPairOpts::builder().dir(Path::new(".")).build())?;
        }

        let tempdir = if let Some(ref dir) = self.tempdir {
            TempDir::new_in(dir).into_diagnostic()?
        } else {
            TempDir::new().into_diagnostic()?
        };
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
                .output(
                    tempdir
                        .path()
                        .join(blue_build_utils::generate_containerfile_path(recipe)?),
                )
                .skip_validation(self.skip_validation)
                .maybe_platform(self.platform)
                .recipe(recipe)
                .drivers(self.drivers)
                .build()
                .try_run()
        })?;

        self.start(&recipe_paths, tempdir.path())
    }
}

impl BuildCommand {
    fn start(&self, recipe_paths: &[PathBuf], temp_dir: &Path) -> Result<()> {
        use rayon::prelude::*;

        trace!("BuildCommand::build_image()");

        let images = recipe_paths
            .par_iter()
            .try_fold(Vec::new, |mut images, recipe_path| -> Result<Vec<String>> {
                images.extend(self.build(
                    recipe_path,
                    &temp_dir.join(blue_build_utils::generate_containerfile_path(recipe_path)?),
                )?);
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

    fn build(&self, recipe_path: &Path, containerfile: &Path) -> Result<Vec<String>> {
        let recipe = Recipe::parse(recipe_path)?;
        let tags = Driver::generate_tags(
            GenerateTagsOpts::builder()
                .oci_ref(&recipe.base_image_ref()?)
                .maybe_alt_tags(recipe.alt_tags.as_deref())
                .maybe_platform(self.platform)
                .build(),
        )?;
        let image_name = self.image_name(&recipe)?;
        let image: Reference = format!("{image_name}:{}", tags.first().map_or("latest", |tag| tag))
            .parse()
            .into_diagnostic()?;
        let cache_image = (self.cache_layers && self.push).then(|| {
            let cache_image = Reference::with_tag(
                image.registry().to_string(),
                image.repository().to_string(),
                format!(
                    "{}-cache",
                    image.tag().expect("Reference should be built with tag")
                ),
            );
            debug!("Using {cache_image} for caching layers");
            cache_image
        });

        if self.push {
            Driver::login(image.registry())?;
            Driver::signing_login(image.registry())?;
        }

        let images = if self.rechunk {
            self.rechunk(
                containerfile,
                &recipe,
                &tags,
                &image_name,
                cache_image.as_ref(),
            )?
        } else if let Some(archive_dir) = self.archive.as_ref() {
            Driver::build_tag_push(
                BuildTagPushOpts::builder()
                    .containerfile(containerfile)
                    .maybe_platform(self.platform)
                    .image(&ImageRef::from(PathBuf::from(format!(
                        "{}/{}.{ARCHIVE_SUFFIX}",
                        archive_dir.to_string_lossy().trim_end_matches('/'),
                        recipe.name.to_lowercase().replace('/', "_"),
                    ))))
                    .squash(self.squash)
                    .maybe_cache_from(cache_image.as_ref())
                    .maybe_cache_to(cache_image.as_ref())
                    .secrets(&recipe.get_secrets())
                    .build(),
            )?
        } else {
            Driver::build_tag_push(
                BuildTagPushOpts::builder()
                    .image(&ImageRef::from(&image))
                    .containerfile(containerfile)
                    .maybe_platform(self.platform)
                    .tags(&tags)
                    .push(self.push)
                    .retry_push(self.retry_push)
                    .retry_count(self.retry_count)
                    .compression(self.compression_format)
                    .squash(self.squash)
                    .maybe_cache_from(cache_image.as_ref())
                    .maybe_cache_to(cache_image.as_ref())
                    .secrets(&recipe.get_secrets())
                    .build(),
            )?
        };

        if self.push && !self.no_sign {
            Driver::sign_and_verify(
                SignVerifyOpts::builder()
                    .image(&image)
                    .retry_push(self.retry_push)
                    .retry_count(self.retry_count)
                    .maybe_platform(self.platform)
                    .build(),
            )?;
        }

        Ok(images)
    }

    fn rechunk(
        &self,
        containerfile: &Path,
        recipe: &Recipe<'_>,
        tags: &[String],
        image_name: &str,
        cache_image: Option<&Reference>,
    ) -> Result<Vec<String>, miette::Error> {
        use blue_build_process_management::drivers::{
            InspectDriver, RechunkDriver,
            opts::{GetMetadataOpts, RechunkOpts},
        };
        let base_image: Reference = format!("{}:{}", &recipe.base_image, &recipe.image_version)
            .parse()
            .into_diagnostic()?;
        let base_digest =
            &Driver::get_metadata(GetMetadataOpts::builder().image(&base_image).build())?;
        let base_digest = base_digest.digest();

        let default_labels = generate_default_labels(recipe)?;
        let labels = recipe.generate_labels(&default_labels);

        Driver::rechunk(
            RechunkOpts::builder()
                .image(image_name)
                .containerfile(containerfile)
                .maybe_platform(self.platform)
                .tags(tags)
                .push(self.push)
                .version(&format!(
                    "{version}.<date>",
                    version = Driver::get_os_version()
                        .oci_ref(&recipe.base_image_ref()?)
                        .call()?,
                ))
                .retry_push(self.retry_push)
                .retry_count(self.retry_count)
                .compression(self.compression_format)
                .base_digest(base_digest)
                .repo(&Driver::get_repo_url()?)
                .name(&recipe.name)
                .description(&recipe.description)
                .base_image(&format!("{}:{}", &recipe.base_image, &recipe.image_version))
                .maybe_tempdir(self.tempdir.as_deref())
                .clear_plan(self.rechunk_clear_plan)
                .maybe_cache_from(cache_image)
                .maybe_cache_to(cache_image)
                .secrets(&recipe.get_secrets())
                .labels(&labels)
                .build(),
        )
    }

    fn image_name(&self, recipe: &Recipe) -> Result<String> {
        let image_name = Driver::generate_image_name(
            GenerateImageNameOpts::builder()
                .name(recipe.name.trim())
                .maybe_registry(self.credentials.registry.as_deref())
                .maybe_registry_namespace(self.registry_namespace.as_deref())
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
}
