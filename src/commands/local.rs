use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use blue_build_recipe::Recipe;
use blue_build_utils::constants::{ARCHIVE_SUFFIX, LOCAL_BUILD};
use clap::Args;
use log::{debug, info, trace};
use miette::{bail, IntoDiagnostic, Result};
use typed_builder::TypedBuilder;
use users::{Users, UsersCache};

use crate::commands::build::BuildCommand;

use super::{BlueBuildCommand, DriverArgs};

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

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct UpgradeCommand {
    #[clap(flatten)]
    common: LocalCommonArgs,
}

impl BlueBuildCommand for UpgradeCommand {
    fn try_run(&mut self) -> Result<()> {
        trace!("UpgradeCommand::try_run()");

        check_can_run()?;

        let recipe = Recipe::parse(&self.common.recipe)?;

        let build = BuildCommand::builder();

        #[cfg(feature = "multi-recipe")]
        let build = build.recipe(vec![self.common.recipe.clone()]);

        #[cfg(not(feature = "multi-recipe"))]
        let build = build.recipe(self.common.recipe.clone());

        let mut build = build
            .archive(LOCAL_BUILD)
            .drivers(self.common.drivers)
            .force(self.common.force)
            .build();

        let image_name = recipe.name.to_lowercase().replace('/', "_");

        clean_local_build_dir(&image_name, false)?;
        debug!("Image name is {image_name}");

        build.try_run()?;

        let status = if self.common.reboot {
            info!("Upgrading image {image_name} and rebooting");

            trace!("rpm-ostree upgrade --reboot");
            Command::new("rpm-ostree")
                .arg("upgrade")
                .arg("--reboot")
                .status()
                .into_diagnostic()?
        } else {
            info!("Upgrading image {image_name}");

            trace!("rpm-ostree upgrade");
            Command::new("rpm-ostree")
                .arg("upgrade")
                .status()
                .into_diagnostic()?
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

        let recipe = Recipe::parse(&self.common.recipe)?;

        let build = BuildCommand::builder();

        #[cfg(feature = "multi-recipe")]
        let build = build.recipe(vec![self.common.recipe.clone()]);

        #[cfg(not(feature = "multi-recipe"))]
        let build = build.recipe(self.common.recipe.clone());

        let mut build = build
            .archive(LOCAL_BUILD)
            .drivers(self.common.drivers)
            .force(self.common.force)
            .build();

        let image_name = recipe.name.to_lowercase().replace('/', "_");
        clean_local_build_dir(&image_name, true)?;
        debug!("Image name is {image_name}");

        build.try_run()?;
        let rebase_url = format!(
            "ostree-unverified-image:oci-archive:{LOCAL_BUILD}/{image_name}.{ARCHIVE_SUFFIX}"
        );

        let status = if self.common.reboot {
            info!("Rebasing image {image_name} and rebooting");

            trace!("rpm-ostree rebase --reboot {rebase_url}");
            Command::new("rpm-ostree")
                .arg("rebase")
                .arg("--reboot")
                .arg(rebase_url)
                .status()
                .into_diagnostic()?
        } else {
            info!("Rebasing image {image_name}");

            trace!("rpm-ostree rebase {rebase_url}");
            Command::new("rpm-ostree")
                .arg("rebase")
                .arg(rebase_url)
                .status()
                .into_diagnostic()?
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

        let entries = fs::read_dir(LOCAL_BUILD).into_diagnostic()?;

        for entry in entries {
            let entry = entry.into_diagnostic()?;
            let path = entry.path();
            trace!("Found {}", path.display());

            if path.is_file() && path.ends_with(ARCHIVE_SUFFIX) {
                if !rebase && path == image_file_path {
                    debug!("Not rebasing, keeping {}", image_file_path.display());
                    continue;
                }
                trace!("Removing {}", path.display());
                fs::remove_file(path).into_diagnostic()?;
            }
        }
    } else {
        debug!(
            "Creating build output dir at {}",
            local_build_path.display()
        );
        fs::create_dir_all(local_build_path).into_diagnostic()?;
    }

    Ok(())
}
