use std::{fs, path::PathBuf};

use blue_build_utils::string_vec;
use clap::{Args, Subcommand, ValueEnum};
use miette::{bail, Context, IntoDiagnostic, Result};
use oci_distribution::Reference;
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

    #[arg(short, long)]
    output_dir: PathBuf,

    #[arg(short = 'V', long)]
    variant: GenIsoVariant,

    #[arg(
        long,
        default_value = "https://github.com/ublue-os/bazzite/raw/main/secure_boot.der"
    )]
    secure_boot_url: String,

    #[arg(long, default_value = "universalblue")]
    enrollment_password: String,

    #[arg(long)]
    iso_name: Option<String>,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GenIsoSubcommand {
    Image {
        #[arg()]
        image: String,
    },
    Recipe {
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

        if self.output_dir.exists() && !self.output_dir.is_dir() {
            bail!("The '--output-dir' arg must be a directory");
        }

        if let GenIsoSubcommand::Recipe { recipe } = &self.command {
            todo!()
        }

        let iso_name = self
            .iso_name
            .as_ref()
            .map_or_else(|| "build.iso", |name| name.as_str());
        let iso_path = self.output_dir.join(iso_name);

        if !self.output_dir.exists() {
            fs::create_dir(&self.output_dir).into_diagnostic()?;
        }

        if iso_path.exists() {
            fs::remove_file(iso_path).into_diagnostic()?;
        }

        let mut args = string_vec![
            format!("VARIANT={}", self.variant),
            format!("ISO_NAME={iso_name}"),
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

                args.extend([
                    format!(
                        "IMAGE_NAME={}/{}",
                        image.resolve_registry(),
                        image.repository()
                    ),
                    format!("IMAGE_TAG={}", image.tag().unwrap_or("latest")),
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
            .volumes(run_volumes! [
                self.output_dir.display().to_string() => "/build-container-installer/build",
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
