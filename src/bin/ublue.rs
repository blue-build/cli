use anyhow::Result;
use clap::Parser;
use ublue_rs::{self, CommandArgs, UblueArgs};

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
        #[cfg(init)]
        CommandArgs::Init { dir } => {
            let base_dir = match dir {
                Some(dir) => dir,
                None => std::path::PathBuf::from("./"),
            };

            ublue_rs::init::initialize_directory(base_dir);
        }
        #[cfg(build)]
        CommandArgs::Build { containerfile: _ } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
