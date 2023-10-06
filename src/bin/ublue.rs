use anyhow::Result;
use clap::Parser;
use ublue_rs::{setup_tera, CommandArgs, UblueArgs};

fn main() -> Result<()> {
    let args = UblueArgs::parse();

    match args.command {
        CommandArgs::Template {
            recipe,
            containerfile,
            output,
        } => {
            let (tera, context) = setup_tera(recipe, containerfile)?;
            let output_str = tera.render("Containerfile", &context)?;
            if let Some(output) = output {
                std::fs::write(output, output_str)?;
            } else {
                println!("{output_str}");
            }
        }
        CommandArgs::Build { containerfile: _ } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
