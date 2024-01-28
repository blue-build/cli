#![warn(clippy::nursery)]

use clap::{command, crate_authors, Parser, Subcommand};

use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::WriteStyle;

use blue_build::{
    self,
    commands::{bug_report, build, local, template, BlueBuildCommand},
};

#[cfg(feature = "init")]
use blue_build::commands::init;

#[macro_use]
extern crate shadow_rs;

shadow!(shadow);

#[derive(Parser, Debug)]
#[clap(
    name = "BlueBuild",
    about,
    long_about = None,
    author=crate_authors!(),
    version=shadow::PKG_VERSION,
    long_version=shadow::CLAP_LONG_VERSION,
    arg_required_else_help=true,
)]
struct BlueBuildArgs {
    #[command(subcommand)]
    command: CommandArgs,

    #[clap(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

#[derive(Debug, Subcommand)]
enum CommandArgs {
    /// Create a pre-populated GitHub issue with information about your configuration
    BugReport,

    // /// Generate starship shell completions for your shell to stdout
    // Completions {
    //     #[clap(value_enum)]
    //     shell: CompletionShell,
    // },
    /// Build an image from a recipe
    Build(build::BuildCommand),

    /// Generate a Containerfile from a recipe
    Template(template::TemplateCommand),

    /// Upgrade your current OS with the
    /// local image saved at `/etc/blue-build/`.
    ///
    /// This requires having rebased already onto
    /// a local archive already by using the `rebase`
    /// subcommand.
    ///
    /// NOTE: This can only be used if you have `rpm-ostree`
    /// installed and if the `--push` and `--rebase` option isn't
    /// used. This image will not be signed.
    Upgrade(local::UpgradeCommand),

    /// Rebase your current OS onto the image
    /// being built.
    ///
    /// This will create a tarball of your image at
    /// `/etc/blue-build/` and invoke `rpm-ostree` to
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
}

fn main() {
    let args = BlueBuildArgs::parse();

    env_logger::builder()
        .filter_level(args.verbosity.log_level_filter())
        .filter_module("hyper::proto", log::LevelFilter::Info)
        .write_style(WriteStyle::Always)
        .init();

    log::trace!("Parsed arguments: {:#?}", args);

    match args.command {
        #[cfg(feature = "init")]
        CommandArgs::Init(mut command) => command.run(),

        #[cfg(feature = "init")]
        CommandArgs::New(mut command) => command.run(),

        CommandArgs::Build(mut command) => command.run(),
        CommandArgs::Rebase(mut command) => command.run(),
        CommandArgs::Upgrade(mut command) => command.run(),
        CommandArgs::Template(mut command) => command.run(),
        CommandArgs::BugReport => bug_report::create(),
    }
}
