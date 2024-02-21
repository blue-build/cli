use std::{
    io::{Read, Seek, SeekFrom, Write},
    process::Command,
};

use anyhow::{anyhow, Result};
use format_serde_error::SerdeError;
use log::{debug, trace};

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

pub fn get_last_char(file_path: &str) -> Result<char> {
    trace!("get_last_char({file_path})");
    debug!("Getting last character of {file_path}");

    let mut last_char_buffer = vec![0; 4096];
    let mut file = std::fs::File::open(file_path)?;

    // get the last character
    file.seek(SeekFrom::Start(file.metadata()?.len() - 1))?;
    file.read_exact(&mut last_char_buffer[0..1])?;

    Ok(String::from_utf8_lossy(&last_char_buffer)
        .to_string()
        .chars()
        .next()
        .unwrap_or('\0'))
}

pub fn append_to_file(file_path: &str, content: &str) -> Result<()> {
    trace!("append_to_file({file_path}, {content})");
    debug!("Appending {content} to {file_path}");

    let last_char = get_last_char(file_path)?;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    // Check if last character is a newline
    if last_char != 0xA as char {
        file.write_all(b"\n")?;
    }

    file.write_all(format!("{}\n", content).as_bytes())?;
    Ok(())
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
