use log::error;

use clap::{command, crate_authors, Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell as CompletionShell};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use env_logger::WriteStyle;

use crate::commands::BlueBuildArgs;

use super::BlueBuildCommand;

#[derive(Debug, Clone, Args)]
pub struct CompletionsCommand {
    #[arg(value_enum)]
    shell: CompletionShell,
}

impl BlueBuildCommand for CompletionsCommand {
    fn try_run(&mut self) -> anyhow::Result<()> {
        log::debug!("Generating completions for {shell}", shell = self.shell);

        generate(
            self.shell,
            &mut BlueBuildArgs::command(),
            "bb",
            &mut std::io::stdout().lock(),
        );

        Ok(())
    }
}
