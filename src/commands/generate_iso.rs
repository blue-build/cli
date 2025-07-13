use std::{
    env, fs,
    path::{self, Path, PathBuf},
};

use blue_build_recipe::Recipe;
use blue_build_utils::{
    constants::{ARCHIVE_SUFFIX, BB_SKIP_VALIDATION},
    string_vec,
    traits::CowCollecter,
};
use bon::Builder;
use clap::{Args, Subcommand, ValueEnum};
use miette::{Context, IntoDiagnostic, Result, bail};
use oci_distribution::Reference;
use tempfile::TempDir;

use blue_build_process_management::{
    drivers::{Driver, DriverArgs, RunDriver, opts::RunOpts},
    run_volumes,
};

use super::{BlueBuildCommand, build::BuildCommand};

#[derive(Clone, Debug, Builder, Args)]
pub struct GenerateIsoCommand {
    #[command(subcommand)]
    command: GenIsoSubcommand,

    /// The directory to save the resulting ISO file.
    #[arg(short, long)]
    #[builder(into)]
    output_dir: Option<PathBuf>,

    /// The variant of the installer to use.
    ///
    /// The Kinoite variant will ask for a user
    /// and password before installing the OS.
    /// This version is the most stable and is
    /// recommended.
    ///
    /// The Silverblue variant will ask for a user
    /// and password on first boot after the OS
    /// is installed.
    ///
    /// The Server variant is the basic installer
    /// and will ask to setup a user at install time.
    #[arg(short = 'V', long, default_value = "kinoite")]
    variant: GenIsoVariant,

    /// The url to the secure boot public key.
    ///
    /// Defaults to one of UBlue's public key.
    /// It's recommended to change this if your base
    /// image is not from UBlue.
    #[arg(
        long,
        default_value = "https://github.com/ublue-os/bazzite/raw/main/secure_boot.der"
    )]
    #[builder(into)]
    secure_boot_url: String,

    /// The enrollment password for the secure boot
    /// key.
    ///
    /// Default's to UBlue's enrollment password.
    /// It's recommended to change this if your base
    /// image is not from UBlue.
    #[arg(long, default_value = "universalblue")]
    #[builder(into)]
    enrollment_password: String,

    /// The name of your ISO image file.
    #[arg(long)]
    #[builder(into)]
    iso_name: Option<String>,

    /// The location to temporarily store files
    /// while building. If unset, it will use `/tmp`.
    #[arg(long)]
    tempdir: Option<PathBuf>,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GenIsoSubcommand {
    /// Build an ISO from a remote image.
    Image {
        /// The image ref to create the iso from.
        #[arg()]
        image: String,
    },
    /// Build an ISO from a recipe.
    ///
    /// This will build the image locally first
    /// before creating the ISO. This is a long
    /// process.
    Recipe {
        /// The path to the recipe file for your image.
        #[arg()]
        recipe: PathBuf,

        /// Skips validation of the recipe file.
        #[arg(long, env = BB_SKIP_VALIDATION)]
        skip_validation: bool,
    },
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum GenIsoVariant {
    #[default]
    Kinoite,
    Silverblue,
    Server,
}

impl std::fmt::Display for GenIsoVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Server => "Server",
                Self::Silverblue => "Silverblue",
                Self::Kinoite => "Kinoite",
            }
        )
    }
}

impl BlueBuildCommand for GenerateIsoCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::init(self.drivers);

        let image_out_dir = if let Some(ref dir) = self.tempdir {
            TempDir::new_in(dir).into_diagnostic()?
        } else {
            TempDir::new().into_diagnostic()?
        };

        let output_dir = if let Some(output_dir) = self.output_dir.clone() {
            if output_dir.exists() && !output_dir.is_dir() {
                bail!("The '--output-dir' arg must be a directory");
            }

            if !output_dir.exists() {
                fs::create_dir(&output_dir).into_diagnostic()?;
            }

            path::absolute(output_dir).into_diagnostic()?
        } else {
            env::current_dir().into_diagnostic()?
        };

        if let GenIsoSubcommand::Recipe {
            recipe,
            skip_validation,
        } = &self.command
        {
            BuildCommand::builder()
                .recipe(vec![recipe.clone()])
                .archive(image_out_dir.path())
                .maybe_tempdir(self.tempdir.clone())
                .skip_validation(*skip_validation)
                .build()
                .try_run()?;
        }

        let iso_name = self.iso_name.as_ref().map_or("deploy.iso", String::as_str);
        let iso_path = output_dir.join(iso_name);

        if iso_path.exists() {
            fs::remove_file(iso_path).into_diagnostic()?;
        }

        self.build_iso(iso_name, &output_dir, image_out_dir.path())
    }
}

impl GenerateIsoCommand {
    fn build_iso(&self, iso_name: &str, output_dir: &Path, image_out_dir: &Path) -> Result<()> {
        let mut args = string_vec![
            format!("VARIANT={}", self.variant),
            format!("ISO_NAME=build/{iso_name}"),
            "DNF_CACHE=/cache/dnf",
            format!("SECURE_BOOT_KEY_URL={}", self.secure_boot_url),
            format!("ENROLLMENT_PASSWORD={}", self.enrollment_password),
        ];
        let mut vols = run_volumes![
            output_dir.display().to_string() => "/build-container-installer/build",
            "dnf-cache" => "/cache/dnf/",
        ];

        match &self.command {
            GenIsoSubcommand::Image { image } => {
                let image: Reference = image
                    .parse()
                    .into_diagnostic()
                    .with_context(|| format!("Unable to parse image reference {image}"))?;
                let (image_repo, image_name) = {
                    let registry = image.resolve_registry();
                    let repo = image.repository();
                    let image = format!("{registry}/{repo}");

                    let mut image_parts = image.split('/').collect::<Vec<_>>();
                    let image_name = image_parts.pop().unwrap(); // Should be at least 2 elements
                    let image_repo = image_parts.join("/");
                    (image_repo, image_name.to_string())
                };

                args.extend([
                    format!("IMAGE_NAME={image_name}",),
                    format!("IMAGE_REPO={image_repo}"),
                    format!("IMAGE_TAG={}", image.tag().unwrap_or("latest")),
                    format!(
                        "VERSION={}",
                        Driver::get_os_version().oci_ref(&image).call()?
                    ),
                ]);
            }
            GenIsoSubcommand::Recipe {
                recipe,
                skip_validation: _,
            } => {
                let recipe = Recipe::parse(recipe)?;

                args.extend([
                    format!(
                        "IMAGE_SRC=oci-archive:/img_src/{}.{ARCHIVE_SUFFIX}",
                        recipe.name.replace('/', "_"),
                    ),
                    format!(
                        "VERSION={}",
                        Driver::get_os_version()
                            .oci_ref(&recipe.base_image_ref()?)
                            .call()?,
                    ),
                ]);
                vols.extend(run_volumes![
                    image_out_dir.display().to_string() => "/img_src/",
                ]);
            }
        }

        // Currently testing local tarball builds
        let opts = RunOpts::builder()
            .image("ghcr.io/jasonn3/build-container-installer")
            .privileged(true)
            .remove(true)
            .args(args.collect_cow_vec())
            .volumes(vols)
            .build();

        let status = Driver::run(&opts)?;

        if !status.success() {
            bail!("Failed to create ISO");
        }
        Ok(())
    }
}
