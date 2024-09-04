use blue_build::commands::{BlueBuildArgs, BlueBuildCommand, CommandArgs};
use blue_build_process_management::{logging::Logger, signal_handler};
use clap::Parser;
use log::LevelFilter;

fn main() {
    let args = BlueBuildArgs::parse();

    Logger::new()
        .filter_level(args.verbosity.log_level_filter())
        .filter_modules([
            ("hyper::proto", LevelFilter::Off),
            ("hyper_util", LevelFilter::Off),
            ("oci_distribution", LevelFilter::Off),
        ])
        .log_out_dir(args.log_out.clone())
        .init();
    log::trace!("Parsed arguments: {args:#?}");

    signal_handler::init(|| match args.command {
        // #[cfg(feature = "init")]
        // CommandArgs::Init(mut command) => command.run(),

        // #[cfg(feature = "init")]
        // CommandArgs::New(mut command) => command.run(),
        CommandArgs::Build(mut command) => command.run(),

        CommandArgs::Generate(mut command) => command.run(),

        #[cfg(feature = "switch")]
        CommandArgs::Switch(mut command) => command.run(),

        #[cfg(not(feature = "switch"))]
        CommandArgs::Rebase(mut command) => command.run(),

        #[cfg(not(feature = "switch"))]
        CommandArgs::Upgrade(mut command) => command.run(),

        #[cfg(feature = "login")]
        CommandArgs::Login(mut command) => command.run(),

        #[cfg(feature = "iso")]
        CommandArgs::GenerateIso(mut command) => command.run(),

        CommandArgs::BugReport(mut command) => command.run(),

        CommandArgs::Completions(mut command) => command.run(),
    });
}
