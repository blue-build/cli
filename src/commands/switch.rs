use std::path::PathBuf;

use blue_build_process_management::drivers::{
    BootDriver, CiDriver, Driver, ImageStorageDriver, PodmanDriver,
    opts::{GenerateImageNameOpts, RemoveImageOpts, SwitchOpts},
};
use blue_build_recipe::{Recipe, RecipeGetters};
use blue_build_utils::constants::BB_SKIP_VALIDATION;
use bon::Builder;
use clap::Args;
use log::trace;
use miette::{Result, bail};

use super::BlueBuildCommand;

#[cfg(not(feature = "next-builder"))]
use super::generate::GenerateCommand;
#[cfg(not(feature = "next-builder"))]
use blue_build_process_management::drivers::{
    BuildDriver, DriverArgs,
    opts::BuildOpts,
    types::{BuildDriverType, RunDriverType},
};
#[cfg(not(feature = "next-builder"))]
use blue_build_utils::{container::ImageRef, tempdir, tempdir_in};

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
    #[cfg(not(feature = "next-builder"))]
    drivers: DriverArgs,
}

impl BlueBuildCommand for SwitchCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("SwitchCommand::try_run()");

        #[cfg(not(feature = "next-builder"))]
        Driver::init(
            DriverArgs::builder()
                .build_driver(BuildDriverType::Podman)
                .run_driver(RunDriverType::Podman)
                .maybe_boot_driver(self.drivers.boot_driver)
                .maybe_signing_driver(self.drivers.signing_driver)
                .build(),
        );

        #[cfg(feature = "next-builder")]
        Driver::init();

        let status = Driver::status()?;

        if status.transaction_in_progress() {
            bail!("There is a transaction in progress. Please cancel it using `rpm-ostree cancel`");
        }

        let recipe = Recipe::builder().path(&self.recipe).build()?;
        let image_name = Driver::generate_image_name(
            GenerateImageNameOpts::builder()
                .name(recipe.get_name().trim())
                .build(),
        )?;
        #[cfg(not(feature = "next-builder"))]
        let tempdir = if let Some(ref dir) = self.tempdir {
            tempdir_in(dir)?
        } else {
            tempdir()?
        };
        #[cfg(not(feature = "next-builder"))]
        let containerfile = tempdir
            .path()
            .join(blue_build_utils::generate_containerfile_path(&self.recipe)?);

        #[cfg(not(feature = "next-builder"))]
        GenerateCommand::builder()
            .output(&containerfile)
            .recipe(&self.recipe)
            .build()
            .try_run()?;
        #[cfg(not(feature = "next-builder"))]
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
