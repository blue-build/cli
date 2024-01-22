#[cfg(feature = "init")]
pub mod init;

pub mod build;
pub mod local;
pub mod template;

pub trait BlueBuildCommand {
    fn run(&mut self);

    /// # Errors
    ///
    /// Will return `Err` - Add to Me :)
    fn try_run(&mut self) -> anyhow::Result<()>;
}
