use std::path::PathBuf;

use blue_build_process_management::drivers::{
    BootDriver, BuildDriver, CiDriver, Driver, DriverArgs, PodmanDriver, RunDriver,
    opts::{BuildOpts, GenerateImageNameOpts, RemoveImageOpts, SwitchOpts},
    types::{BuildDriverType, ImageRef, RunDriverType},
};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::BB_SKIP_VALIDATION;
use bon::Builder;
use clap::Args;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};
use tempfile::TempDir;

use crate::commands::generate::GenerateCommand;

use super::BlueBuildCommand;

#[derive(Default, Clone, Debug, Builder, Args)]
pub struct SwitchCommand {
    /// The recipe file to build an image.
    #[arg()]
    recipe: PathBuf,

    /// Reboot your system after
    /// the update is complete.
    #[arg(short, long)]
    #[builder(default)]
    reboot: bool,

    /// The location to temporarily store files
    /// while building. If unset, it will use `/tmp`.
    #[arg(long)]
    tempdir: Option<PathBuf>,

    /// Skips validation of the recipe file.
    #[arg(long, env = BB_SKIP_VALIDATION)]
    #[builder(default)]
    skip_validation: bool,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for SwitchCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("SwitchCommand::try_run()");

        Driver::init(
            DriverArgs::builder()
                .build_driver(BuildDriverType::Podman)
                .run_driver(RunDriverType::Podman)
                .maybe_boot_driver(self.drivers.boot_driver)
                .maybe_signing_driver(self.drivers.signing_driver)
                .build(),
        );

        let status = Driver::status()?;

        if status.transaction_in_progress() {
            bail!("There is a transaction in progress. Please cancel it using `rpm-ostree cancel`");
        }

        let recipe = Recipe::parse(&self.recipe)?;
        let image_name = Driver::generate_image_name(
            GenerateImageNameOpts::builder()
                .name(recipe.name.trim())
                .build(),
        )?;
        let tempdir = if let Some(ref dir) = self.tempdir {
            TempDir::new_in(dir).into_diagnostic()?
        } else {
            TempDir::new().into_diagnostic()?
        };
        let containerfile = tempdir
            .path()
            .join(blue_build_utils::generate_containerfile_path(&self.recipe)?);

        GenerateCommand::builder()
            .output(&containerfile)
            .recipe(&self.recipe)
            .build()
            .try_run()?;
        Driver::build(
            BuildOpts::builder()
                .image(&ImageRef::from(&image_name))
                .containerfile(&containerfile)
                .secrets(&recipe.get_secrets())
                .build(),
        )?;
        PodmanDriver::copy_image_to_root_store(&image_name)?;
        PodmanDriver::remove_image(RemoveImageOpts::builder().image(&image_name).build())?;

        if status
            .booted_image()
            .is_some_and(|booted| booted == image_name)
        {
            Driver::upgrade(
                SwitchOpts::builder()
                    .image(&image_name)
                    .reboot(self.reboot)
                    .build(),
            )
        } else {
            Driver::switch(
                SwitchOpts::builder()
                    .image(&image_name)
                    .reboot(self.reboot)
                    .build(),
            )
        }
    }
}
