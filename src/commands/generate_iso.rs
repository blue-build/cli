use std::{
    env, fs,
    path::{self, PathBuf},
};

use blue_build_utils::string_vec;
use clap::{Args, Subcommand, ValueEnum};
use miette::{bail, Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
use tempdir::TempDir;
use typed_builder::TypedBuilder;

use blue_build_process_management::{
    drivers::{opts::RunOpts, Driver, DriverArgs, RunDriver},
    run_volumes,
};

use super::{build::BuildCommand, BlueBuildCommand};

#[derive(Clone, Debug, TypedBuilder, Args)]
pub struct GenerateIsoCommand {
    #[command(subcommand)]
    command: GenIsoSubcommand,

    /// The directory to save the resulting ISO file.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// The variant of the installer to use.
    ///
    /// The Gnome variant will ask for a user
    /// and password on first boot after the OS
    /// is installed.
    ///
    /// The Kinoite variant will ask for a user
    /// and password before installing the OS.
    ///
    /// The Server variant is more useful for
    /// images built for a server like UCore.
    #[arg(short = 'V', long)]
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
    secure_boot_url: String,

    /// The enrollment password for the secure boot
    /// key.
    ///
    /// Default's to UBlue's enrollment password.
    /// It's recommended to change this if your base
    /// image is not from UBlue.
    #[arg(long, default_value = "universalblue")]
    enrollment_password: String,

    /// The name of your ISO image file.
    #[arg(long)]
    iso_name: Option<String>,

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
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GenIsoVariant {
    Gnome,
    Kinoite,
    Server,
}

impl std::fmt::Display for GenIsoVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Gnome => "Gnome",
                Self::Kinoite => "Kinoite",
                Self::Server => "Server",
            }
        )
    }
}

impl BlueBuildCommand for GenerateIsoCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::init(self.drivers);

        let image_out_dir = TempDir::new("build_image").into_diagnostic()?;

        let output_dir = if let Some(output_dir) = self.output_dir.clone() {
            if output_dir.exists() && !output_dir.is_dir() {
                bail!("The '--output-dir' arg must be a directory");
            }

            if !output_dir.exists() {
                fs::create_dir(&output_dir).into_diagnostic()?;
            }

            output_dir
        } else {
            env::current_dir().into_diagnostic()?
        };

        if let GenIsoSubcommand::Recipe { recipe } = &self.command {
            // BuildCommand::builder().recipe(

            // )
            todo!()
        }

        let iso_name = self
            .iso_name
            .as_ref()
            .map_or_else(|| "deploy.iso", |name| name.as_str());
        let iso_path = output_dir.join(iso_name);

        if iso_path.exists() {
            fs::remove_file(iso_path).into_diagnostic()?;
        }

        let mut args = string_vec![
            format!("VARIANT={}", self.variant),
            format!("ISO_NAME=build/{iso_name}"),
            "DNF_CACHE=/cache/dnf",
            format!("SECURE_BOOT_KEY_URL={}", self.secure_boot_url),
            format!("ENROLLMENT_PASSWORD={}", self.enrollment_password),
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
                    format!("VERSION={}", Driver::get_os_version(&image)?),
                ]);
            }
            GenIsoSubcommand::Recipe { recipe } => {
                todo!()
            }
        }

        // Currently testing local tarball builds
        let opts = RunOpts::builder()
            .image("ghcr.io/jasonn3/build-container-installer")
            .privileged(true)
            .remove(true)
            .args(&args)
            .volumes(run_volumes![
                path::absolute(output_dir).into_diagnostic()?.display().to_string() => "/build-container-installer/build",
                "dnf-cache" => "/cache/dnf/",
            ])
            .build();

        let status = Driver::run(&opts).into_diagnostic()?;

        if !status.success() {
            bail!("Failed to create ISO");
        }
        Ok(())
    }
}
