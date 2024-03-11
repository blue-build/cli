use log::error;

use clap::{command, crate_authors, Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

use crate::shadow;

pub mod bug_report;
pub mod build;
pub mod completions;
pub mod generate;
#[cfg(feature = "init")]
pub mod init;

pub trait BlueBuildCommand {
    /// Runs the command and returns a result
    /// of the execution
    ///
    /// # Errors
    /// Can return an `anyhow` Error
    fn try_run(&mut self) -> anyhow::Result<()>;

    /// Runs the command and exits if there is an error.
    fn run(&mut self) {
        if let Err(e) = self.try_run() {
            error!("{e}");
            std::process::exit(1);
        }
    }
}

#[derive(Parser, Debug)]
#[clap(
    name = "BlueBuild",
    about,
    long_about = None,
    author=crate_authors!(),
    version=shadow::PKG_VERSION,
    long_version=shadow::CLAP_LONG_VERSION,
)]
pub struct BlueBuildArgs {
    #[command(subcommand)]
    pub command: CommandArgs,

    #[clap(flatten)]
    pub verbosity: Verbosity<InfoLevel>,
}

#[derive(Debug, Subcommand)]
pub enum CommandArgs {
    /// Build an image from a recipe
    Build(build::BuildCommand),

    /// Generate a Containerfile from a recipe
    Generate(generate::GenerateCommand),

    /// Initialize a new Ublue Starting Point repo
    #[cfg(feature = "init")]
    Init(init::InitCommand),

    #[cfg(feature = "init")]
    New(init::NewCommand),

    /// Create a pre-populated GitHub issue with information about your configuration
    BugReport(bug_report::BugReportCommand),

    /// Generate shell completions for your shell to stdout
    Completions(completions::CompletionsCommand),
}
