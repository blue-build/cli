use std::{
    ffi::OsStr,
    fmt::Debug,
    io::{BufRead, BufReader, Error, ErrorKind, Result},
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use colored::Colorize;
use process_control::{ChildExt, Control};
use rand::Rng;

pub trait CommandExt {
    /// Prints each line of stdout with a prefix string
    /// that is given a random color.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn status_log_prefix<T: AsRef<str>>(&mut self, log_prefix: &T) -> Result<ExitStatus>;
}

impl CommandExt for Command {
    fn status_log_prefix<T: AsRef<str>>(&mut self, log_prefix: &T) -> Result<ExitStatus> {
        let mut child = self.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        let mut rng = rand::thread_rng();
        let red: u8 = rng.gen_range(80..=240);
        let green: u8 = rng.gen_range(80..=240);
        let blue: u8 = rng.gen_range(80..=240);

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let prefix = log_prefix.as_ref().truecolor(red, green, blue);

            thread::spawn(move || {
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        eprintln!("{prefix} {seperator} {l}", seperator = "=>".bold());
                    }
                });
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            let prefix = log_prefix.as_ref().truecolor(red, green, blue);

            thread::spawn(move || {
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        eprintln!("{prefix} {seperator} {l}", seperator = "=>".bold());
                    }
                });
            });
        }

        child.wait()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

/// # Attempt to resolve `binary_name` from and creates a new `Command` pointing at it
/// # This allows executing cmd files on Windows and prevents running executable from cwd on Windows
/// # This function also initializes std{err,out,in} to protect against processes changing the console mode
/// #
/// # Errors
///
fn create_command<T: AsRef<OsStr>>(binary_name: T) -> Result<Command> {
    let binary_name = binary_name.as_ref();
    log::trace!("Creating Command for binary {:?}", binary_name);

    let full_path = match which::which(binary_name) {
        Ok(full_path) => {
            log::trace!("Using {:?} as {:?}", full_path, binary_name);
            full_path
        }
        Err(error) => {
            log::trace!("Unable to find {:?} in PATH, {:?}", binary_name, error);
            return Err(Error::new(ErrorKind::NotFound, error));
        }
    };

    let mut cmd = Command::new(full_path);
    cmd.stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .stdin(Stdio::null());

    Ok(cmd)
}

/// Execute a command and return the output on stdout and stderr if successful
pub fn exec_cmd<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
    time_limit: Duration,
) -> Option<CommandOutput> {
    log::trace!("Executing command {:?} with args {:?}", cmd, args);
    internal_exec_cmd(cmd, args, time_limit)
}

fn internal_exec_cmd<T: AsRef<OsStr> + Debug, U: AsRef<OsStr> + Debug>(
    cmd: T,
    args: &[U],
    time_limit: Duration,
) -> Option<CommandOutput> {
    let mut cmd = create_command(cmd).ok()?;
    cmd.args(args);
    exec_timeout(&mut cmd, time_limit)
}

fn exec_timeout(cmd: &mut Command, time_limit: Duration) -> Option<CommandOutput> {
    let start = Instant::now();
    let process = match cmd.spawn() {
        Ok(process) => process,
        Err(error) => {
            log::trace!("Unable to run {:?}, {:?}", cmd.get_program(), error);
            return None;
        }
    };
    match process
        .controlled_with_output()
        .time_limit(time_limit)
        .terminate_for_timeout()
        .wait()
    {
        Ok(Some(output)) => {
            let stdout_string = match String::from_utf8(output.stdout) {
                Ok(stdout) => stdout,
                Err(error) => {
                    log::warn!("Unable to decode stdout: {:?}", error);
                    return None;
                }
            };
            let stderr_string = match String::from_utf8(output.stderr) {
                Ok(stderr) => stderr,
                Err(error) => {
                    log::warn!("Unable to decode stderr: {:?}", error);
                    return None;
                }
            };

            log::trace!(
                "stdout: {:?}, stderr: {:?}, exit code: \"{:?}\", took {:?}",
                stdout_string,
                stderr_string,
                output.status.code(),
                start.elapsed()
            );

            if !output.status.success() {
                return None;
            }

            Some(CommandOutput {
                stdout: stdout_string,
                stderr: stderr_string,
            })
        }
        Ok(None) => {
            log::warn!("Executing command {:?} timed out.", cmd.get_program());
            log::warn!("You can set command_timeout in your config to a higher value to allow longer-running commands to keep executing.");
            None
        }
        Err(error) => {
            log::trace!(
                "Executing command {:?} failed by: {:?}",
                cmd.get_program(),
                error
            );
            None
        }
    }
}
