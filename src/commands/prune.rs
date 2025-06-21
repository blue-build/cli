use blue_build_process_management::drivers::{BuildDriver, Driver, DriverArgs, opts::PruneOpts};
use bon::Builder;
use clap::Args;
use colored::Colorize;
use miette::bail;

use super::BlueBuildCommand;

#[derive(Debug, Args, Builder)]
pub struct PruneCommand {
    /// Remove all unused images
    #[builder(default)]
    #[arg(short, long)]
    all: bool,

    /// Do not prompt for confirmation
    #[builder(default)]
    #[arg(short, long)]
    force: bool,

    /// Prune volumes
    #[builder(default)]
    #[arg(long)]
    volumes: bool,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for PruneCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        Driver::init(self.drivers);

        if !self.force {
            eprintln!(
                "{} This will remove:{default}{images}{build_cache}{volumes}",
                "WARNING!".bright_yellow(),
                default = concat!(
                    "\n - all stopped containers",
                    "\n - all networks not used by at least one container",
                ),
                images = if self.all {
                    "\n - all images without at least one container associated to them"
                } else {
                    "\n - all dangling images"
                },
                build_cache = if self.all {
                    "\n - all build cache"
                } else {
                    "\n - unused build cache"
                },
                volumes = if self.volumes {
                    "\n - all anonymous volumes not used by at least one container"
                } else {
                    ""
                },
            );

            match requestty::prompt_one(
                requestty::Question::confirm("anonymous")
                    .message("Are you sure you want to continue?")
                    .default(false)
                    .build(),
            ) {
                Err(e) => bail!("Canceled {e:?}"),
                Ok(answer) => {
                    if answer.as_bool().is_some_and(|a| !a) {
                        return Ok(());
                    }
                }
            }
        }

        Driver::prune(
            PruneOpts::builder()
                .all(self.all)
                .volumes(self.volumes)
                .build(),
        )
    }
}
