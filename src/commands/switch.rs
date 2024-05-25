use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::{ARCHIVE_SUFFIX, LOCAL_BUILD};
use clap::Args;
use log::{debug, info, trace};
use typed_builder::TypedBuilder;
use users::{Users, UsersCache};

use crate::commands::build::BuildCommand;

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

        check_can_run()?;

        let recipe = Recipe::parse(&self.recipe)?;

        let mut build = BuildCommand::builder()
            .recipe(self.recipe.clone())
            .archive(LOCAL_BUILD)
            .drivers(self.drivers)
            .force(self.force)
            .build();

        let image_name = recipe.name.to_lowercase().replace('/', "_");

        clean_local_build_dir(&image_name, false)?;
        debug!("Image name is {image_name}");

        build.try_run()?;

        let status = if self.reboot {
            info!("Upgrading image {image_name} and rebooting");

            trace!("rpm-ostree upgrade --reboot");
            Command::new("rpm-ostree")
                .arg("upgrade")
                .arg("--reboot")
                .status()?
        } else {
            info!("Upgrading image {image_name}");

            trace!("rpm-ostree upgrade");
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

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //

fn check_can_run() -> Result<()> {
    trace!("check_can_run()");

    blue_build_utils::check_command_exists("rpm-ostree")?;

    let cache = UsersCache::new();
    if cache.get_current_uid() != 0 {
        bail!("You need to be root to rebase a local image! Try using 'sudo'.");
    }
    Ok(())
}

fn clean_local_build_dir(image_name: &str, rebase: bool) -> Result<()> {
    trace!("clean_local_build_dir()");

    let local_build_path = Path::new(LOCAL_BUILD);
    let image_file_path = local_build_path.join(format!("{image_name}.{ARCHIVE_SUFFIX}"));

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
