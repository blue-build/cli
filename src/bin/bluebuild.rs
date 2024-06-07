use blue_build::commands::{BlueBuildArgs, BlueBuildCommand, CommandArgs};
use blue_build_utils::logging::Logger;
use clap::Parser;
use log::LevelFilter;

fn main() {
    let args = BlueBuildArgs::parse();

    Logger::new()
        .filter_level(args.verbosity.log_level_filter())
        .filter_modules([("hyper::proto", LevelFilter::Info)])
        .log_out_dir(args.log_out.clone())
        .init();

    log::trace!("Parsed arguments: {args:#?}");

    match args.command {
        #[cfg(feature = "init")]
        CommandArgs::Init(mut command) => command.run(),

        #[cfg(feature = "init")]
        CommandArgs::New(mut command) => command.run(),

        CommandArgs::Build(mut command) => command.run(),

        CommandArgs::Generate(mut command) => command.run(),

        #[cfg(feature = "switch")]
        CommandArgs::Switch(mut command) => command.run(),

        #[cfg(not(feature = "switch"))]
        CommandArgs::Rebase(mut command) => command.run(),

        #[cfg(not(feature = "switch"))]
        CommandArgs::Upgrade(mut command) => command.run(),

        CommandArgs::BugReport(mut command) => command.run(),

        CommandArgs::Completions(mut command) => command.run(),
    }
}
