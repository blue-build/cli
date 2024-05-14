use blue_build::commands::{BlueBuildArgs, BlueBuildCommand, CommandArgs};
use blue_build_utils::logging;
use clap::Parser;
use log::LevelFilter;

fn main() {
    let args = BlueBuildArgs::parse();

    env_logger::builder()
        .filter_level(args.verbosity.log_level_filter())
        .filter_module("hyper::proto", LevelFilter::Info)
        .format(logging::format_log)
        .init();

    log::trace!("Parsed arguments: {args:#?}");

    match args.command {
        #[cfg(feature = "init")]
        CommandArgs::Init(mut command) => command.run(),
        #[cfg(feature = "init")]
        CommandArgs::New(mut command) => command.run(),
        CommandArgs::Build(mut command) => command.run(),
        CommandArgs::Rebase(mut command) => command.run(),
        CommandArgs::Upgrade(mut command) => command.run(),
        CommandArgs::Template(mut command) => command.run(),
        CommandArgs::BugReport(mut command) => command.run(),
        CommandArgs::Completions(mut command) => command.run(),
    }
}
