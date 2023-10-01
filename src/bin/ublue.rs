use anyhow::Result;
use clap::Parser;
use ublue_rs::{setup_tera, CommandArgs, UblueArgs};

fn main() -> Result<()> {
    let args = UblueArgs::parse();

    match args.command {
        CommandArgs::Template {
            recipe,
            containerfile: _,
        } => {
            let (tera, context) = setup_tera(recipe)?;
            let output = tera.render("Containerfile", &context)?;
            println!("{output}");
        }
        CommandArgs::Build { containerfile: _ } => {
            println!("Not yet implemented!");
            todo!();
        }
    }
    Ok(())
}
