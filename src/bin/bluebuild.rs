use blue_build::commands::*;
use clap::Parser;
use env_logger::WriteStyle;

fn main() {
    let args = BlueBuildArgs::parse();

    env_logger::builder()
        .filter_level(args.verbosity.log_level_filter())
        .filter_module("hyper::proto", log::LevelFilter::Info)
        .write_style(WriteStyle::Always)
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
