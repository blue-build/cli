#[cfg(feature = "init")]
pub mod init;

pub mod build;
pub mod local;
pub mod template;

use log::error;

pub trait BlueBuildCommand {
    fn try_run(&mut self) -> anyhow::Result<()>;

    /// Runs the command and exits if there is an error.
    fn run(&mut self) {
        if let Err(e) = self.try_run() {
            error!("{e}");
            std::process::exit(1);
        }
    }
}
