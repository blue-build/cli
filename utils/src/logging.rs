use std::{
    io::{BufRead, BufReader, Result, Write},
    process::{Command, ExitStatus},
    sync::Arc,
    thread,
};

use chrono::Local;
use colored::{control::ShouldColorize, ColoredString, Colorize};
use env_logger::fmt::Formatter;
use indicatif::MultiProgress;
use indicatif_log_bridge::LogWrapper;
use log::{Level, LevelFilter, Record};
use nu_ansi_term::Color;
use once_cell::sync::Lazy;
use rand::Rng;

static MULTI_PROGRESS: Lazy<MultiProgress> = Lazy::new(MultiProgress::new);

pub struct Logger {
    modules: Vec<(String, LevelFilter)>,
    level: LevelFilter,
}

impl Logger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter_modules<I, S>(&mut self, filter_modules: I) -> &mut Self
    where
        I: IntoIterator<Item = (S, LevelFilter)>,
        S: AsRef<str>,
    {
        self.modules = filter_modules
            .into_iter()
            .map(|(module, level)| (module.as_ref().to_string(), level))
            .collect::<Vec<_>>();
        self
    }

    pub fn filter_level(&mut self, filter_level: LevelFilter) -> &mut Self {
        self.level = filter_level;
        self
    }

    /// Initializes logging for the application.
    ///
    /// # Panics
    /// Will panic if logging is unable to be initialized.
    pub fn init(&mut self) {
        let mut logger_builder = env_logger::builder();
        logger_builder.format(format_log).filter_level(self.level);

        self.modules.iter().for_each(|(module, level)| {
            logger_builder.filter_module(module, *level);
        });

        let logger = logger_builder.build();

        LogWrapper::new(MULTI_PROGRESS.clone(), logger)
            .try_init()
            .expect("LogWrapper should initialize");
    }

    pub fn multi_progress() -> MultiProgress {
        MULTI_PROGRESS.clone()
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            modules: vec![],
            level: LevelFilter::Info,
        }
    }
}

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
    fn status_log_prefix<T: AsRef<str>>(self, log_prefix: T) -> Result<ExitStatus>;
}

impl CommandLogging for Command {
    fn status_log_prefix<T: AsRef<str>>(mut self, log_prefix: T) -> Result<ExitStatus> {
        // ANSI extended color range
        // https://www.ditig.com/publications/256-colors-cheat-sheet
        const LOW_END: u8 = 21; // Blue1 #0000ff rgb(0,0,255) hsl(240,100%,50%)
        const HIGH_END: u8 = 230; // Cornsilk1 #ffffd7 rgb(255,255,215) hsl(60,100%,92%)

        let log_prefix = Arc::new(log_header(
            if ShouldColorize::from_env().should_colorize() {
                Color::Fixed(rand::thread_rng().gen_range(LOW_END..=HIGH_END))
                    .paint(log_prefix.as_ref().to_string())
                    .to_string()
            } else {
                log_prefix.as_ref().to_string()
            },
        ));

        let (reader, writer) = os_pipe::pipe()?;

        self.stdout(writer.try_clone()?).stderr(writer);

        let mut child = self.spawn()?;

        drop(self);

        let reader = BufReader::new(reader);

        thread::spawn(move || {
            reader.lines().for_each(|line| {
                if let Ok(l) = line {
                    eprintln!("{log_prefix} {l}");
                }
            });
        });

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
