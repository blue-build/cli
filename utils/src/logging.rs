use std::{
    io::{BufRead, BufReader, Result, Write},
    process::{Command, ExitStatus, Stdio},
    sync::Arc,
    thread,
};

use chrono::Local;
use colored::{control::ShouldColorize, ColoredString, Colorize};
use env_logger::fmt::Formatter;
use log::{Level, LevelFilter, Record};
use nu_ansi_term::Color;
use rand::Rng;

trait ColoredLevel {
    fn colored(&self) -> ColoredString;
}

impl ColoredLevel for Level {
    fn colored(&self) -> ColoredString {
        match self {
            Self::Error => Self::Error.as_str().red(),
            Self::Warn => Self::Warn.as_str().yellow(),
            Self::Info => Self::Info.as_str().green(),
            Self::Debug => Self::Debug.as_str().blue(),
            Self::Trace => Self::Trace.as_str().cyan(),
        }
    }
}

pub trait CommandLogging {
    /// Prints each line of stdout with a prefix string
    /// that is given a random color.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn status_log_prefix<T: AsRef<str>>(&mut self, log_prefix: T) -> Result<ExitStatus>;
}

impl CommandLogging for Command {
    fn status_log_prefix<T: AsRef<str>>(&mut self, log_prefix: T) -> Result<ExitStatus> {
        let mut rng = rand::thread_rng();
        let ansi_color: u8 = rng.gen_range(21..=230);
        let log_prefix = Arc::new(log_header(
            if ShouldColorize::from_env().should_colorize() {
                Color::Fixed(ansi_color)
                    .paint(log_prefix.as_ref().to_string())
                    .to_string()
            } else {
                log_prefix.as_ref().to_string()
            },
        ));

        let mut child = self.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let log_prefix = log_prefix.clone();

            thread::spawn(move || {
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        eprintln!("{log_prefix} {l}");
                    }
                });
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);

            thread::spawn(move || {
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        eprintln!("{log_prefix} {l}");
                    }
                });
            });
        }

        child.wait()
    }
}

/// Given a `LevelFilter`, returns the function
/// used to format logs. The more verbose the log level,
/// the more info is displayed in each log header.
///
/// # Errors
/// Errors if the buffer cannot be written to.
pub fn format_log(buf: &mut Formatter, record: &Record) -> Result<()> {
    match log::max_level() {
        LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => {
            writeln!(
                buf,
                "{prefix} {args}",
                prefix = log_header(format!(
                    "{level:width$}",
                    level = record.level().colored(),
                    width = 5,
                )),
                args = record.args(),
            )
        }
        LevelFilter::Debug => writeln!(
            buf,
            "{prefix} {args}",
            prefix = log_header(format!(
                "{level:>width$}",
                level = record.level().colored(),
                width = 5,
            )),
            args = record.args(),
        ),
        LevelFilter::Trace => writeln!(
            buf,
            "{prefix} {args}",
            prefix = log_header(format!(
                "{level:width$} {module}:{line}",
                level = record.level().colored(),
                width = 5,
                module = record
                    .module_path()
                    .map_or_else(|| "", |p| p)
                    .bright_yellow(),
                line = record
                    .line()
                    .map_or_else(String::new, |l| l.to_string())
                    .bright_green(),
            )),
            args = record.args(),
        ),
        LevelFilter::Off => Ok(()),
    }
}

/// Used to keep the style of logs consistent between
/// normal log use and command output.
fn log_header<T: AsRef<str>>(text: T) -> String {
    let text = text.as_ref();
    match log::max_level() {
        LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => {
            format!("{text} {sep}", sep = "=>".bold())
        }
        LevelFilter::Debug | LevelFilter::Trace => format!(
            "[{time} {text}] {sep}",
            time = Local::now().format("%H:%M:%S"),
            sep = "=>".bold(),
        ),
        LevelFilter::Off => String::new(),
    }
}

/// Shortens the image name so that it won't take up the
/// entire width of the terminal. This is a similar format
/// to what Earthly does in their terminal output for long
/// images on their log prefix output.
///
/// # Examples
/// `ghcr.io/blue-build/cli:latest` -> `g.i/b/cli:latest`
/// `registry.gitlab.com/some/namespace/image:latest` -> `r.g.c/s/n/image:latest`
#[must_use]
pub fn shorten_image_names(text: &str) -> String {
    // Split the reference by colon to separate the tag or digest
    let mut parts = text.split(':');

    let path = match parts.next() {
        None => return text.to_string(),
        Some(path) => path,
    };
    let tag = parts.next();

    // Split the path by slash to work on each part
    let path_parts: Vec<&str> = path.split('/').collect();

    // Shorten each part except the last one to their initial letters
    let shortened_parts: Vec<String> = path_parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if i < path_parts.len() - 1 {
                // Split on '.' and shorten each section
                part.split('.')
                    .filter_map(|p| p.chars().next())
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(".")
            } else {
                (*part).into() // Keep the last part as it is
            }
        })
        .collect();

    // Rejoin the parts with '/'
    let joined_path = shortened_parts.join("/");

    // If there was a tag, append it back with ':', otherwise just return the path
    match tag {
        Some(t) => format!("{joined_path}:{t}"),
        None => joined_path,
    }
}
