use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use blue_build_process_management::{
    drivers::{Driver, DriverArgs},
    logging::CommandLogging,
};
use blue_build_recipe::Recipe;
use blue_build_utils::{
    cmd,
    constants::{ARCHIVE_SUFFIX, LOCAL_BUILD, OCI_ARCHIVE, OSTREE_UNVERIFIED_IMAGE},
};
use bon::Builder;
use clap::Args;
use colored::Colorize;
use indicatif::ProgressBar;
use log::{debug, trace, warn};
use miette::{bail, IntoDiagnostic, Result};
use tempfile::TempDir;

use crate::{commands::build::BuildCommand, rpm_ostree_status::RpmOstreeStatus};

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

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for SwitchCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("SwitchCommand::try_run()");

        Driver::init(self.drivers);

        let status = RpmOstreeStatus::try_new()?;
        trace!("{status:?}");

        if status.transaction_in_progress() {
            bail!("There is a transaction in progress. Please cancel it using `rpm-ostree cancel`");
        }

        let tempdir = TempDir::new().into_diagnostic()?;
        trace!("{tempdir:?}");

        #[cfg(feature = "multi-recipe")]
        BuildCommand::builder()
            .recipe([self.recipe.clone()])
            .archive(tempdir.path())
            .build()
            .try_run()?;
        #[cfg(not(feature = "multi-recipe"))]
        BuildCommand::builder()
            .recipe(self.recipe.clone())
            .archive(tempdir.path())
            .build()
            .try_run()?;

        let recipe = Recipe::parse(&self.recipe)?;
        let image_file_name = format!(
            "{}.{ARCHIVE_SUFFIX}",
            recipe.name.to_lowercase().replace('/', "_")
        );
        let temp_file_path = tempdir.path().join(&image_file_name);
        let archive_path = Path::new(LOCAL_BUILD).join(&image_file_name);

        warn!(
            "{notice}: {} {sudo} {}",
            "The next few steps will require".yellow(),
            "You may have to supply your password".yellow(),
            notice = "NOTICE".bright_red().bold(),
            sudo = "`sudo`.".italic().bright_red().bold(),
        );
        Self::sudo_clean_local_build_dir()?;
        Self::sudo_move_archive(&temp_file_path, &archive_path)?;

        // We drop the tempdir ahead of time so that the directory
        // can be cleaned out.
        drop(tempdir);

        self.switch(&archive_path, &status)
    }
}

impl SwitchCommand {
    fn switch(&self, archive_path: &Path, status: &RpmOstreeStatus<'_>) -> Result<()> {
        trace!(
            "SwitchCommand::switch({}, {status:#?})",
            archive_path.display()
        );

        let status = if status.is_booted_on_archive(archive_path)
            || status.is_staged_on_archive(archive_path)
        {
            let mut command = cmd!("rpm-ostree", "upgrade");

            if self.reboot {
                cmd!(command, "--reboot");
            }

            trace!("{command:?}");
            command
        } else {
            let image_ref = format!(
                "{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{path}",
                path = archive_path.display()
            );

            let mut command = cmd!("rpm-ostree", "rebase", &image_ref);

            if self.reboot {
                cmd!(command, "--reboot");
            }

            trace!("{command:?}");
            command
        }
        .build_status(
            format!("{}", archive_path.display()),
            "Switching to new image",
        )
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to switch to new image!");
        }
        Ok(())
    }

    fn sudo_move_archive(from: &Path, to: &Path) -> Result<()> {
        trace!(
            "SwitchCommand::sudo_move_archive({}, {})",
            from.display(),
            to.display()
        );

        let progress = ProgressBar::new_spinner();
        progress.enable_steady_tick(Duration::from_millis(100));
        progress.set_message(format!("Moving image archive to {}...", to.display()));

        trace!("sudo mv {} {}", from.display(), to.display());
        let status = cmd!("sudo", "mv", from, to).status().into_diagnostic()?;

        progress.finish_and_clear();

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
        trace!("SwitchCommand::clean_local_build_dir()");

        let local_build_path = Path::new(LOCAL_BUILD);

        if local_build_path.exists() {
            debug!("Cleaning out build dir {LOCAL_BUILD}");

            trace!("sudo ls {LOCAL_BUILD}");
            let output = String::from_utf8(
                cmd!("sudo", "ls", LOCAL_BUILD)
                    .output()
                    .into_diagnostic()?
                    .stdout,
            )
            .into_diagnostic()?;

            trace!("{output}");

            let files = output
                .lines()
                .filter(|line| line.ends_with(ARCHIVE_SUFFIX))
                .map(|file| local_build_path.join(file).display().to_string())
                .collect::<Vec<_>>();

            if !files.is_empty() {
                let files = files.join(" ");

                let progress = ProgressBar::new_spinner();
                progress.enable_steady_tick(Duration::from_millis(100));
                progress.set_message("Removing old image archive files...");

                trace!("sudo rm -f {files}");
                let status = cmd!("sudo", "rm", "-f", files).status().into_diagnostic()?;

                progress.finish_and_clear();

                if !status.success() {
                    bail!("Failed to clean out archives in {LOCAL_BUILD}");
                }
            }
        } else {
            debug!(
                "Creating build output dir at {}",
                local_build_path.display()
            );

            let status = cmd!("sudo", "mkdir", "-p", LOCAL_BUILD)
                .status()
                .into_diagnostic()?;

            if !status.success() {
                bail!("Failed to create directory {LOCAL_BUILD}");
            }
        }

        Ok(())
    }
}
