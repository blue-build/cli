use blue_build::commands::{BlueBuildArgs, BlueBuildCommand, CommandArgs};
use blue_build_process_management::{logging::Logger, signal_handler};
use clap::Parser;
use colored::Colorize;
use log::LevelFilter;

fn main() {
    let args = BlueBuildArgs::parse();

    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .footer(format!(
                    concat!(
                        "Try running with `{arg}` for trace logging.\n",
                        "You can also open an issue at https://github.com/blue-build/cli/issues"
                    ),
                    arg = "-vv".bright_yellow(),
                ))
                .context_lines(3)
                .tab_width(4)
                .wrap_lines(false)
                .with_cause_chain()
                .show_related_errors_as_nested()
                .build(),
        )
    }))
    .expect("Miette hook should be set");

    Logger::new()
        .filter_level(args.verbosity.log_level_filter())
        .filter_modules(if args.no_log_filter {
            vec![]
        } else {
            vec![
                ("hyper::proto", LevelFilter::Off),
                ("hyper_util", LevelFilter::Off),
                ("reqwest", LevelFilter::Off),
                ("oci_client", LevelFilter::Off),
                ("rustls", LevelFilter::Off),
                ("mio", LevelFilter::Off),
            ]
        })
        .log_out_dir(args.log_out.clone())
        .init();
    log::trace!("Parsed arguments: {args:#?}");

    signal_handler::init(|| match args.command {
        CommandArgs::Build(mut command) => command.run(),
        CommandArgs::Generate(mut command) => command.run(),
        CommandArgs::Switch(mut command) => command.run(),
        CommandArgs::Login(mut command) => command.run(),
        CommandArgs::New(mut command) => command.run(),
        CommandArgs::Init(mut command) => command.run(),
        CommandArgs::GenerateIso(mut command) => command.run(),
        CommandArgs::Validate(mut command) => command.run(),
        CommandArgs::Prune(mut command) => command.run(),
        CommandArgs::BugReport(mut command) => command.run(),
        CommandArgs::Completions(mut command) => command.run(),
    });
}
