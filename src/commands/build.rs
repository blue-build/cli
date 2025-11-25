use std::{
    num::NonZeroU32,
    ops::Not,
    path::{Path, PathBuf},
};

use blue_build_process_management::{
    drivers::{
        BuildChunkedOciDriver, BuildDriver, CiDriver, Driver, DriverArgs, InspectDriver,
        RechunkDriver, SigningDriver,
        opts::{
            BuildChunkedOciOpts, BuildRechunkTagPushOpts, BuildTagPushOpts, CheckKeyPairOpts,
            CompressionType, GenerateImageNameOpts, GenerateTagsOpts, GetMetadataOpts, RechunkOpts,
            SignVerifyOpts,
        },
        types::{BuildDriverType, RunDriverType},
    },
    logging::{color_str, gen_random_ansi_color},
};
use blue_build_recipe::Recipe;
use blue_build_utils::{
    constants::{
        ARCHIVE_SUFFIX, BB_BUILD_ARCHIVE, BB_BUILD_CHUNKED_OCI, BB_BUILD_CHUNKED_OCI_MAX_LAYERS,
        BB_BUILD_NO_SIGN, BB_BUILD_PLATFORM, BB_BUILD_PUSH, BB_BUILD_RECHUNK,
        BB_BUILD_RECHUNK_CLEAR_PLAN, BB_BUILD_RETRY_COUNT, BB_BUILD_RETRY_PUSH, BB_BUILD_SQUASH,
        BB_CACHE_LAYERS, BB_REGISTRY_NAMESPACE, BB_SKIP_VALIDATION, BB_TEMPDIR, CONFIG_PATH,
        DEFAULT_MAX_LAYERS, RECIPE_FILE, RECIPE_PATH,
    },
    container::{ImageRef, Tag},
    credentials::{Credentials, CredentialsArgs},
    platform::Platform,
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

    /// Build for specific platforms.
    ///
    /// This will override any platform setting in
    /// the recipes you're building.
    ///
    /// NOTE: Building for a different architecture
    /// than your hardware will require installing
    /// qemu. Build times will be much greater when
    /// building for a non-native architecture.
    #[builder(default)]
    #[arg(long, env = BB_BUILD_PLATFORM)]
    platform: Vec<Platform>,

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

    /// Uses `rpm-ostree compose build-chunked-oci` to rechunk the image,
    /// allowing for smaller images and smaller updates.
    ///
    /// WARN: This will increase the build-time
    /// and take up more space during build-time.
    #[arg(long, env = BB_BUILD_CHUNKED_OCI)]
    #[builder(default)]
    build_chunked_oci: bool,

    /// Maximum number of layers to use when rechunking. Requires `--build-chunked-oci`.
    #[arg(
        long,
        default_value_t = DEFAULT_MAX_LAYERS,
        env = BB_BUILD_CHUNKED_OCI_MAX_LAYERS,
        requires = "build_chunked_oci"
    )]
    #[builder(default = DEFAULT_MAX_LAYERS)]
    max_layers: NonZeroU32,

    /// Uses `hhd-dev/rechunk` to rechunk the image, allowing for smaller images
    /// and smaller updates.
    ///
    /// WARN: This will be deprecated in the future.
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

        Driver::init(if self.build_chunked_oci || self.rechunk {
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

        if self.rechunk && self.build_chunked_oci {
            bail!("You cannot use '--rechunk' and '--build-chunked-oci' at the same time");
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
                .maybe_platform(self.platform.first().copied())
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
        trace!(
            "BuildCommand::start({recipe_paths:?}, {})",
            temp_dir.display()
        );

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

    #[expect(clippy::too_many_lines)]
    fn build(&self, recipe_path: &Path, containerfile: &Path) -> Result<Vec<String>> {
        trace!(
            "BuildCommand::build({}, {})",
            recipe_path.display(),
            containerfile.display()
        );

        let recipe = &Recipe::parse(recipe_path)?;
        let tags = &Driver::generate_tags(
            GenerateTagsOpts::builder()
                .oci_ref(&recipe.base_image_ref()?)
                .maybe_alt_tags(recipe.alt_tags.as_deref())
                .maybe_platform(self.platform.first().copied())
                .build(),
        )?;
        assert!(
            tags.is_empty().not(),
            "At least 1 tag must have been generated"
        );

        let image = &Driver::generate_image_name(
            GenerateImageNameOpts::builder()
                .name(recipe.name.trim())
                .maybe_registry(self.credentials.registry.as_deref())
                .maybe_registry_namespace(self.registry_namespace.as_deref())
                .maybe_tag(tags.first())
                .build(),
        )?;

        if self.push {
            Driver::login(image.registry())?;
            Driver::signing_login(image.registry())?;
        }

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
        let cache_image = cache_image.as_ref();

        let platforms = match &recipe.platforms {
            None if self.platform.is_empty() => &vec![Platform::default()],
            Some(platform) if platform.is_empty().not() && self.platform.is_empty() => {
                &platform.clone()
            }
            _ => &self.platform.clone(),
        };
        assert!(
            platforms.is_empty().not(),
            "At least one platform must be built"
        );

        let secrets = &recipe.get_secrets();

        let image_ref = self.archive.as_ref().map_or_else(
            || ImageRef::from(image),
            |archive_dir| {
                ImageRef::from(PathBuf::from(format!(
                    "{}/{}.{ARCHIVE_SUFFIX}",
                    archive_dir.to_string_lossy().trim_end_matches('/'),
                    recipe.name.to_lowercase().replace('/', "_"),
                )))
            },
        );

        let build_tag_opts = BuildTagPushOpts::builder()
            .image(&image_ref)
            .containerfile(containerfile)
            .platform(platforms)
            .squash(self.squash)
            .maybe_cache_from(cache_image)
            .maybe_cache_to(cache_image)
            .secrets(secrets);

        let opts = if matches!(image_ref, ImageRef::Remote(_)) {
            build_tag_opts
                .tags(tags)
                .push(self.push)
                .retry_push(self.retry_push)
                .retry_count(self.retry_count)
                .compression(self.compression_format)
                .build()
        } else {
            build_tag_opts.build()
        };

        let images = if self.build_chunked_oci {
            let rechunk_opts = BuildChunkedOciOpts::builder()
                .max_layers(self.max_layers)
                .build();
            Driver::build_rechunk_tag_push(
                BuildRechunkTagPushOpts::builder()
                    .build_tag_push_opts(opts)
                    .rechunk_opts(rechunk_opts)
                    .build(),
            )?
        } else if self.rechunk {
            self.rechunk(containerfile, recipe, tags, image, cache_image, platforms)?
        } else {
            Driver::build_tag_push(opts)?
        };

        if self.push && !self.no_sign {
            Driver::sign_and_verify(
                SignVerifyOpts::builder()
                    .image(image)
                    .retry_push(self.retry_push)
                    .retry_count(self.retry_count)
                    .platforms(platforms)
                    .build(),
            )?;
        }

        Ok(images)
    }

    fn rechunk(
        &self,
        containerfile: &Path,
        recipe: &Recipe,
        tags: &[Tag],
        image_name: &Reference,
        cache_image: Option<&Reference>,
        platforms: &[Platform],
    ) -> Result<Vec<String>, miette::Error> {
        trace!(
            "BuildCommand::rechunk({}, {recipe:?}, {tags:?}, {image_name}, {cache_image:?}, {platforms:?})",
            containerfile.display()
        );

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
                .platform(platforms)
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
                .base_image(&base_image)
                .maybe_tempdir(self.tempdir.as_deref())
                .clear_plan(self.rechunk_clear_plan)
                .maybe_cache_from(cache_image)
                .maybe_cache_to(cache_image)
                .secrets(&recipe.get_secrets())
                .labels(&labels)
                .build(),
        )
    }
}
