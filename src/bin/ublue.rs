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
        recipe: PathBuf,

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

    /// Build an image from a recipe
    #[cfg(feature = "build")]
    Build {
        /// The recipe file to build an image
        #[arg()]
        recipe: PathBuf,

        /// Optional Containerfile to use as a template
        #[arg(short, long)]
        containerfile: Option<PathBuf>,

        /// Push the image with all the tags.
        ///
        /// Requires `--registry`, `--registry-path`,
        /// `--username`, and `--password` if not
        /// building in CI.
        #[arg(short, long)]
        push: bool,

        /// The registry's domain name.
        #[arg(long)]
        registry: Option<String>,

        /// The url path to your base
        /// project images.
        #[arg(long)]
        registry_path: Option<String>,

        /// The username to login to the
        /// container registry.
        #[arg(short, long)]
        username: Option<String>,

        /// The password to login to the
        /// container registry.
        #[arg(short, long)]
        password: Option<String>,
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
            ublue_rs::template_file(&recipe, containerfile.as_ref(), output.as_ref())?;
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
        CommandArgs::Build {
            recipe,
            containerfile,
            push,
            registry,
            registry_path,
            username,
            password,
        } => {
            ublue_rs::template_file(
                &recipe,
                containerfile.as_ref(),
                Some(&PathBuf::from("Containerfile")),
            )?;
            ublue_rs::build::build_image(
                &recipe,
                registry.as_ref(),
                registry_path.as_ref(),
                username.as_ref(),
                password.as_ref(),
                push,
            )?;
        }
    }
    Ok(())
}
