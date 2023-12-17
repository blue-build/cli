use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use ublue_rs::{self};

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
        recipe: String,

        /// Optional Containerfile to use as a template
        #[arg(short, long)]
        containerfile: Option<PathBuf>,

        /// File to output to instead of STDOUT
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Initialize a new Ublue Starting Point repo
    #[cfg(feature = "init")]
    Init {
        /// The directory to extract the files into. Defaults to the current directory
        #[arg()]
        dir: Option<PathBuf>,
    },

    /// Build an image from a Containerfile
    #[cfg(feature = "build")]
    Build {
        #[arg()]
        containerfile: String,
    },
}

fn main() -> Result<()> {
    let args = UblueArgs::parse();

    match args.command {
        CommandArgs::Template {
            recipe,
            containerfile,
            output,
        } => {
            let (tera, context) = ublue_rs::setup_tera(recipe, containerfile)?;
            let output_str = tera.render("Containerfile", &context)?;

            if let Some(output) = output {
                std::fs::write(output, output_str)?;
            } else {
                println!("{output_str}");
            }
        }
        #[cfg(feature = "init")]
        CommandArgs::Init { dir } => {
            let base_dir = match dir {
                Some(dir) => dir,
                None => std::path::PathBuf::from("./"),
            };

            ublue_rs::init::initialize_directory(base_dir);
        }
        #[cfg(feature = "build")]
        CommandArgs::Build { containerfile: _ } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
