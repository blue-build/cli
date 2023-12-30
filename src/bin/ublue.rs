use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::WriteStyle;
use log::trace;
use ublue_rs::{self, build, template};

#[cfg(feature = "init")]
use ublue_rs::init;

#[derive(Parser, Debug)]
#[command(name = "Ublue Builder", author, version, about, long_about = None)]
struct UblueArgs {
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
    let args = UblueArgs::parse();

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
