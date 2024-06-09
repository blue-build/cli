use std::path::PathBuf;

use clap::Args;
use typed_builder::TypedBuilder;

use super::BlueBuildCommand;

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct GenerateIsoCommand {
    recipe: PathBuf,
}

impl BlueBuildCommand for GenerateIsoCommand {
    fn try_run(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
