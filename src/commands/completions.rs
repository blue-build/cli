use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell as CompletionShell};

use crate::commands::BlueBuildArgs;

use super::BlueBuildCommand;

#[derive(Debug, Clone, Args)]
pub struct CompletionsCommand {
    #[arg(value_enum)]
    shell: CompletionShell,
}

impl BlueBuildCommand for CompletionsCommand {
    fn try_run(&mut self) -> anyhow::Result<()> {
        log::debug!("Generating completions for {}", self.shell);

        generate(
            self.shell,
            &mut BlueBuildArgs::command(),
            "bluebuild",
            &mut std::io::stdout().lock(),
        );

        Ok(())
    }
}
