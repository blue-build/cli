use anyhow::{anyhow, Result};
use format_serde_error::SerdeError;
use log::{debug, trace};
use std::process::{Command, Stdio};

pub fn check_command_exists(command: &str) -> Result<()> {
    trace!("check_command_exists({command})");
    debug!("Checking if {command} exists");

    trace!("which {command}");
    if Command::new("which")
        .arg(command)
        .output()?
        .status
        .success()
    {
        debug!("Command {command} does exist");
        Ok(())
    } else {
        Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        ))
    }
}

pub fn check_file_modified(file: &str) -> Result<bool> {
    trace!("check_file_modified({file})");
    debug!("Checking if {file} is modified");

    // First command: echo "Hello, world!"
    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .spawn()?;

    let git_status_output = git_status.stdout.expect("failed to capture echo stdout");

    // Git returns 0 if the file is not modified, 1 if it is
    let git_status_output = Command::new("grep")
        .arg(file)
        .stdin(Stdio::from(git_status_output))
        .output()?;

    if git_status_output.stdout.is_empty() {
        debug!("{file} is not modified");
        Ok(false)
    } else {
        debug!("{file} is modified");
        Ok(true)
    }
}

pub fn serde_yaml_err(contents: &str) -> impl Fn(serde_yaml::Error) -> SerdeError + '_ {
    |err: serde_yaml::Error| {
        let location = err.location();
        let location = location.as_ref();
        SerdeError::new(
            contents.to_string(),
            (
                err.into(),
                location.map_or(0, serde_yaml::Location::line).into(),
                location.map_or(0, serde_yaml::Location::column).into(),
            ),
        )
    }
}
