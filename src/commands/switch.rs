use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::{ARCHIVE_SUFFIX, LOCAL_BUILD};
use clap::Args;
use log::{debug, trace};
use tempdir::TempDir;
use typed_builder::TypedBuilder;

use crate::{commands::build::BuildCommand, drivers::Driver, rpm_ostree_status::RpmOstreeStatus};

use super::{BlueBuildCommand, DriverArgs};

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct SwitchCommand {
    /// The recipe file to build an image.
    #[arg()]
    recipe: PathBuf,

    /// Reboot your system after
    /// the update is complete.
    #[arg(short, long)]
    #[builder(default)]
    reboot: bool,

    /// Allow `bluebuild` to overwrite an existing
    /// Containerfile without confirmation.
    ///
    /// This is not needed if the Containerfile is in
    /// .gitignore or has already been built by `bluebuild`.
    #[arg(short, long)]
    #[builder(default)]
    force: bool,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for SwitchCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("SwitchCommand::try_run()");

        Driver::builder()
            .build_driver(self.drivers.build_driver)
            .inspect_driver(self.drivers.inspect_driver)
            .build()
            .init()?;

        let status = RpmOstreeStatus::try_new()?;

        if status.transaction_in_progress() {
            bail!("There is a transaction in progress. Please cancel it using `rpm-ostree cancel`");
        }

        let tempdir = TempDir::new("oci-archive")?;

        BuildCommand::builder()
            .recipe(self.recipe.clone())
            .archive(tempdir.path())
            .force(self.force)
            .build()
            .try_run()?;

        let recipe = Recipe::parse(&self.recipe)?;
        let image_file_name = format!(
            "{}.{ARCHIVE_SUFFIX}",
            recipe.name.to_lowercase().replace('/', "_")
        );
        let temp_file_path = tempdir.path().join(&image_file_name);
        let archive_path = Path::new(LOCAL_BUILD).join(&image_file_name);

        Self::sudo_clean_local_build_dir()?;
        Self::sudo_move_archive(&temp_file_path, &archive_path)?;
        self.switch()
    }
}

impl SwitchCommand {
    fn switch(&self) -> Result<()> {
        todo!()
    }

    fn sudo_move_archive(from: &Path, to: &Path) -> Result<()> {
        let status = Command::new("sudo").arg("mv").args([from, to]).status()?;

        if !status.success() {
            bail!(
                "Failed to move archive from {from} to {to}",
                from = from.display(),
                to = to.display()
            );
        }

        Ok(())
    }

    fn sudo_clean_local_build_dir() -> Result<()> {
        trace!("clean_local_build_dir()");

        let local_build_path = Path::new(LOCAL_BUILD);

        if local_build_path.exists() {
            debug!("Cleaning out build dir {LOCAL_BUILD}");

            let status = Command::new("sudo")
                .args(["rm", "-f"])
                .arg(format!("{LOCAL_BUILD}/*.{ARCHIVE_SUFFIX}"))
                .status()?;

            if !status.success() {
                bail!("Failed to clean out archives in {LOCAL_BUILD}");
            }
        } else {
            debug!(
                "Creating build output dir at {}",
                local_build_path.display()
            );

            let status = Command::new("sudo")
                .args(["mkdir", "-p", LOCAL_BUILD])
                .status()?;

            if !status.success() {
                bail!("Failed to create directory {LOCAL_BUILD}");
            }
        }

        Ok(())
    }
}
