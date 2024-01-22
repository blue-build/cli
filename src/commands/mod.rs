#[cfg(feature = "init")]
pub mod init;

pub mod build;
pub mod local;
pub mod template;

pub trait BlueBuildCommand {
    fn try_run(&mut self) -> anyhow::Result<()>;

    fn run(&mut self) {
        if let Err(e) = self.try_run() {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
