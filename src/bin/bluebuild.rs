use anyhow::{anyhow, bail, Result};
use blue_build::commands::{BlueBuildArgs, BlueBuildCommand, CommandArgs};
use blue_build_utils::{home_dir, logging};
use clap::Parser;
use log::LevelFilter;
use tracing_indicatif::IndicatifLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::FormatEvent, layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    let args = BlueBuildArgs::parse();

    // env_logger::builder()
    //     .filter_level(args.verbosity.log_level_filter())
    //     .filter_module("hyper::proto", LevelFilter::Info)
    //     .format(logging::format_log)
    //     .init();

    // let logger = LogTracer::new();
    // log::set_boxed_logger(Box::new(logger))?;
    // log::set_max_level(args.verbosity.log_level_filter());

    let file_appender = tracing_appender::rolling::hourly(
        home_dir()
            .ok_or_else(|| anyhow!("Failed to get home directory"))?
            .join(".local/share/bluebuild"),
        "bluebuild.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let indicatif_layer = IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .with(tracing_subscriber::fmt::layer().with_writer(indicatif_layer.get_stderr_writer()))
        .with(indicatif_layer)
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

    Ok(())
}
