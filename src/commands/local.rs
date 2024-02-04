use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use clap::Args;
use format_serde_error::SerdeError;
use log::{debug, info, trace};
use typed_builder::TypedBuilder;
use users::{Users, UsersCache};

use crate::{
    commands::build::BuildCommand,
    module_recipe::Recipe,
    ops::{self, ARCHIVE_SUFFIX, LOCAL_BUILD},
};

use super::BlueBuildCommand;

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct LocalCommonArgs {
    /// The recipe file to build an image.
    #[arg()]
    recipe: PathBuf,

    /// Reboot your system after
    /// the update is complete.
    #[arg(short, long)]
    #[builder(default)]
    reboot: bool,
}

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct UpgradeCommand {
    #[clap(flatten)]
    common: LocalCommonArgs,
}

impl BlueBuildCommand for UpgradeCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("UpgradeCommand::try_run()");

        check_can_run()?;

        let recipe: Recipe =
            serde_yaml::from_str(fs::read_to_string(&self.common.recipe)?.as_str())
                .map_err(|err| SerdeError::new(self.common.recipe.display().to_string(), err))?;

        let mut build = BuildCommand::builder()
            .recipe(self.common.recipe.clone())
            .archive(LOCAL_BUILD)
            .build();

        let image_name = build.generate_full_image_name(&recipe)?;
        clean_local_build_dir(&image_name, false)?;
        debug!("Image name is {image_name}");

        build.try_run()?;

        let status = if self.common.reboot {
            info!("Upgrading image {image_name} and rebooting");

            Command::new("rpm-ostree")
                .arg("upgrade")
                .arg("--reboot")
                .status()?
        } else {
            info!("Upgrading image {image_name}");

            Command::new("rpm-ostree").arg("upgrade").status()?
        };

        if status.success() {
            info!("Successfully upgraded image {image_name}");
        } else {
            bail!("Failed to upgrade image {image_name}");
        }
        Ok(())
    }
}

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct RebaseCommand {
    #[clap(flatten)]
    common: LocalCommonArgs,
}

impl BlueBuildCommand for RebaseCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("RebaseCommand::try_run()");

        check_can_run()?;

        let recipe: Recipe =
            serde_yaml::from_str(fs::read_to_string(&self.common.recipe)?.as_str())
                .map_err(|err| SerdeError::new(self.common.recipe.display().to_string(), err))?;

        let mut build = BuildCommand::builder()
            .recipe(self.common.recipe.clone())
            .archive(LOCAL_BUILD)
            .build();

        let image_name = build.generate_full_image_name(&recipe)?;
        clean_local_build_dir(&image_name, true)?;
        debug!("Image name is {image_name}");

        build.try_run()?;

        let status = if self.common.reboot {
            info!("Rebasing image {image_name} and rebooting");

            Command::new("rpm-ostree")
                .arg("rebase")
                .arg("--reboot")
                .arg(format!("ostree-unverified-image:{image_name}"))
                .status()?
        } else {
            info!("Rebasing image {image_name}");

            Command::new("rpm-ostree")
                .arg("rebase")
                .arg(format!("ostree-unverified-image:{image_name}"))
                .status()?
        };

        if status.success() {
            info!("Successfully rebased to {image_name}");
        } else {
            bail!("Failed to rebase to {image_name}");
        }
        Ok(())
    }
}

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //

fn check_can_run() -> Result<()> {
    trace!("check_can_run()");

    ops::check_command_exists("rpm-ostree")?;

    let cache = UsersCache::new();
    if cache.get_current_uid() != 0 {
        bail!("You need to be root to rebase a local image! Try using 'sudo'.");
    }
    Ok(())
}

fn clean_local_build_dir(image_name: &str, rebase: bool) -> Result<()> {
    trace!("clean_local_build_dir()");

    let local_build_path = Path::new(LOCAL_BUILD);
    let image_file_path = local_build_path.join(image_name.trim_start_matches("oci-archive:"));

    if !image_file_path.exists() && !rebase {
        bail!(
            "Cannot upgrade {} as the image doesn't exist",
            image_file_path.display()
        );
    }

    if local_build_path.exists() {
        debug!("Cleaning out build dir {LOCAL_BUILD}");

        let entries = fs::read_dir(LOCAL_BUILD)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            trace!("Found {}", path.display());

            if path.is_file() && path.ends_with(ARCHIVE_SUFFIX) {
                if !rebase && path == image_file_path {
                    debug!("Not rebasing, keeping {}", image_file_path.display());
                    continue;
                }
                trace!("Removing {}", path.display());
                fs::remove_file(path)?;
            }
        }
    } else {
        debug!(
            "Creating build output dir at {}",
            local_build_path.display()
        );
        fs::create_dir_all(local_build_path)?;
    }

    Ok(())
}
