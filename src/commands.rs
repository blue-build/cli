use log::error;

use clap::{command, crate_authors, Args, Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use typed_builder::TypedBuilder;

use crate::{
    drivers::types::{BuildDriverType, InspectDriverType},
    shadow,
};

pub mod bug_report;
pub mod build;
pub mod completions;
#[cfg(feature = "init")]
pub mod init;
pub mod local;
pub mod template;

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
    Template(template::TemplateCommand),

    /// Upgrade your current OS with the
    /// local image saved at `/etc/bluebuild/`.
    ///
    /// This requires having rebased already onto
    /// a local archive already by using the `rebase`
    /// subcommand.
    ///
    /// NOTE: This can only be used if you have `rpm-ostree`
    /// installed and if the `--push` and `--rebase` option isn't
    /// used. This image will not be signed.
    #[command(visible_alias("update"))]
    Upgrade(local::UpgradeCommand),

    /// Rebase your current OS onto the image
    /// being built.
    ///
    /// This will create a tarball of your image at
    /// `/etc/bluebuild/` and invoke `rpm-ostree` to
    /// rebase onto the image using `oci-archive`.
    ///
    /// NOTE: This can only be used if you have `rpm-ostree`
    /// installed.
    Rebase(local::RebaseCommand),

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

#[derive(Default, Clone, Copy, Debug, TypedBuilder, Args)]
pub struct DriverArgs {
    /// Puts the build in a `squash-stage` and
    /// COPY's the results to the final stage
    /// as one layer.
    ///
    /// NOTE: This doesn't work with buildkit builders
    /// for docker. You will want to use the standard
    /// builder to use squash.
    ///
    /// NOTE: Squash has a performance benefit for
    /// the newer versions of podman and buildah.
    /// It can also
    #[arg(short, long)]
    #[builder(default)]
    squash: bool,

    /// Select which driver to use to build
    /// your image.
    #[builder(default)]
    #[arg(short = 'B', long)]
    build_driver: Option<BuildDriverType>,

    /// Select which driver to use to inspect
    /// images.
    #[builder(default)]
    #[arg(short = 'I', long)]
    inspect_driver: Option<InspectDriverType>,
}
