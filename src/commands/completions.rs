use clap::{Args, CommandFactory};
use clap_complete::generate;
use miette::Result;
use shells::Shells;

use crate::commands::BlueBuildArgs;

use super::BlueBuildCommand;

mod shells;

#[derive(Debug, Clone, Args)]
pub struct CompletionsCommand {
    #[arg(value_enum)]
    shell: Shells,
}

impl BlueBuildCommand for CompletionsCommand {
    fn try_run(&mut self) -> Result<()> {
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
