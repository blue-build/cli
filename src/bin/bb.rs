use blue_build::{self, build, template};
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::WriteStyle;
use log::trace;

#[cfg(feature = "init")]
use blue_build::init;

#[derive(Parser, Debug)]
#[command(name = "BlueBuild", author, version, about, long_about = None)]
struct BlueBuildArgs {
    #[command(subcommand)]
    command: CommandArgs,

    #[clap(flatten)]
    verbosity: Verbosity<InfoLevel>,
}

#[derive(Debug, Subcommand)]
enum CommandArgs {
    /// Generate a Containerfile from a recipe
    Template(template::TemplateCommand),

    /// Initialize a new Ublue Starting Point repo
    #[cfg(feature = "init")]
    Init(init::InitCommand),

    #[cfg(feature = "init")]
    New(init::NewCommand),

    /// Build an image from a recipe
    #[cfg(feature = "build")]
    Build(build::BuildCommand),
}

fn main() {
    let args = BlueBuildArgs::parse();

    env_logger::builder()
        .filter_level(args.verbosity.log_level_filter())
        .write_style(WriteStyle::Always)
        .init();

    trace!("{args:#?}");

    match args.command {
        CommandArgs::Template(command) => command.run(),

        #[cfg(feature = "init")]
        CommandArgs::Init(command) => command.run(),

        #[cfg(feature = "init")]
        CommandArgs::New(command) => command.run(),

        #[cfg(feature = "build")]
        CommandArgs::Build(command) => command.run(),
    }
}
