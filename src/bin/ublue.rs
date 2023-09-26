use std::{fs, io, path::PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tera::{Context, Tera};
use ublue_rs::{Recipe, DEFAULT_CONTAINERFILE};

#[derive(Parser, Debug)]
#[command(name = "Ublue Builder", author, version, about, long_about = None)]
struct UblueArgs {
    #[command(subcommand)]
    command: CommandArgs,
}

#[derive(Debug, Subcommand)]
enum CommandArgs {
    /// Generate a Containerfile from a recipe
    Template {
        /// The recipe file to create a template from
        #[arg()]
        recipe: PathBuf,

        /// Optional Containerfile to use as a template
        #[arg(short, long)]
        containerfile: Option<PathBuf>,
    },

    /// Build an image from a Containerfile
    Build {
        #[arg()]
        containerfile: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = UblueArgs::parse();

    match args.command {
        CommandArgs::Template {
            recipe,
            containerfile,
        } => {
            let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(recipe)?.as_str())?;
            println!("{:#?}", &recipe);
            let context = Context::from_serialize(recipe)?;
            dbg!(&context);
            let output = Tera::one_off(DEFAULT_CONTAINERFILE, &context, true)?;
            println!("{output}");
        }
        CommandArgs::Build { containerfile } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
