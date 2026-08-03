use bon::Builder;
use clap::Args;

use crate::commands::BlueBuildCommand;

#[derive(Clone, Debug, Builder, Args)]
pub struct GenerateIsoCommand {}

impl BlueBuildCommand for GenerateIsoCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        todo!()
    }
}
