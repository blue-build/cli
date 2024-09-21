use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Result},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::RwLock,
    thread,
    time::Duration,
};

use chrono::Local;
use colored::{control::ShouldColorize, ColoredString, Colorize};
use indicatif::{MultiProgress, ProgressBar};
use indicatif_log_bridge::LogWrapper;
use log::{Level, LevelFilter, Record};
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Root},
    encode::{pattern::PatternEncoder, Encode, Write},
    Config, Logger as L4RSLogger,
};
use nu_ansi_term::Color;
use once_cell::sync::Lazy;
use os_pipe::PipeReader;
use rand::Rng;
use typed_builder::TypedBuilder;

mod command;
mod docker;

pub use command::*;
pub use docker::*;

use crate::signal_handler::{add_pid, remove_pid};

static MULTI_PROGRESS: Lazy<MultiProgress> = Lazy::new(MultiProgress::new);
static LOG_DIR: Lazy<RwLock<PathBuf>> = Lazy::new(|| RwLock::new(PathBuf::new()));

#[derive(Debug, Clone)]
pub struct Logger {
    modules: Vec<(String, LevelFilter)>,
    level: LevelFilter,
    log_dir: Option<PathBuf>,
}

impl Logger {
    const TRIGGER_FILE_SIZE: u64 = 10 * 1024;
    const ARCHIVE_FILENAME_PATTERN: &'static str = "bluebuild-log.{}.log";
    const LOG_FILENAME: &'static str = "bluebuild-log.log";
    const LOG_FILE_COUNT: u32 = 4;

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn filter_modules<I, S>(mut self, filter_modules: I) -> Self
    where
        I: AsRef<[(S, LevelFilter)]>,
        S: ToString,
    {
        self.modules = filter_modules
            .as_ref()
            .iter()
            .map(|(module, level)| (module.to_string(), *level))
            .collect::<Vec<_>>();
        self
    }

    #[must_use]
    pub const fn filter_level(mut self, filter_level: LevelFilter) -> Self {
        self.level = filter_level;
        self
    }

    #[must_use]
    pub fn log_out_dir<P>(mut self, path: Option<P>) -> Self
    where
        P: AsRef<Path>,
    {
        self.log_dir = path.map(|p| p.as_ref().to_path_buf());
        self
    }

    /// Initializes logging for the application.
    ///
    /// # Panics
    /// Will panic if logging is unable to be initialized.
    pub fn init(self) {
        let home = env::var("HOME").expect("$HOME should be defined");
        let log_dir = self.log_dir.as_ref().map_or_else(
            || Path::new(home.as_str()).join(".local/share/bluebuild"),
            Clone::clone,
        );

        let mut lock = LOG_DIR.write().expect("Should lock LOG_DIR");
        lock.clone_from(&log_dir);
        drop(lock);

        let log_out_path = log_dir.join(Self::LOG_FILENAME);
        let log_archive_pattern =
            format!("{}/{}", log_dir.display(), Self::ARCHIVE_FILENAME_PATTERN);

        let stderr = ConsoleAppender::builder()
            .encoder(Box::new(
                CustomPatternEncoder::builder()
                    .filter_modules(self.modules.clone())
                    .build(),
            ))
            .target(log4rs::append::console::Target::Stderr)
            .tty_only(true)
            .build();

        let file = RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}{n}")))
            .build(
                log_out_path,
                Box::new(CompoundPolicy::new(
                    Box::new(SizeTrigger::new(Self::TRIGGER_FILE_SIZE)),
                    Box::new(
                        FixedWindowRoller::builder()
                            .build(&log_archive_pattern, Self::LOG_FILE_COUNT)
                            .expect("Roller should be created"),
                    ),
                )),
            )
            .expect("Must be able to create log FileAppender");

        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .appender(Appender::builder().build("file", Box::new(file)))
            .build(
                Root::builder()
                    .appender("stderr")
                    .appender("file")
                    .build(self.level),
            )
            .expect("Logger config should build");

        let logger = L4RSLogger::new(config);

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
            log_dir: None,
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

#[derive(Debug, TypedBuilder)]
struct CustomPatternEncoder {
    #[builder(default, setter(into))]
    filter_modules: Vec<(String, LevelFilter)>,
}

impl Encode for CustomPatternEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
        if record.module_path().is_some_and(|mp| {
            self.filter_modules
                .iter()
                .any(|(module, level)| mp.contains(module) && *level <= record.level())
        }) {
            Ok(())
        } else {
            match log::max_level() {
                LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => Ok(writeln!(
                    w,
                    "{prefix} {args}",
                    prefix = log_header(format!(
                        "{level:width$}",
                        level = record.level().colored(),
                        width = 5,
                    )),
                    args = record.args(),
                )?),
                LevelFilter::Debug => Ok(writeln!(
                    w,
                    "{prefix} {args}",
                    prefix = log_header(format!(
                        "{level:>width$}",
                        level = record.level().colored(),
                        width = 5,
                    )),
                    args = record.args(),
                )?),
                LevelFilter::Trace => Ok(writeln!(
                    w,
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
                )?),
                LevelFilter::Off => Ok(()),
            }
        }
    }
}

trait ProgressLogger {
    /// Prints each line of stdout/stderr with an image ref string
    /// and a progress spinner. This helps to keep track of every
    /// build running in parallel.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn progress_log<N, M, F>(
        self,
        color: u8,
        name: N,
        message: M,
        log_handler: F,
    ) -> Result<ExitStatus>
    where
        N: AsRef<str>,
        M: AsRef<str>,
        F: FnOnce(BufReader<PipeReader>, BufWriter<File>) + Send + 'static;
}

impl ProgressLogger for Command {
    fn progress_log<I, M, F>(
        mut self,
        color: u8,
        name: I,
        message: M,
        log_handler: F,
    ) -> Result<ExitStatus>
    where
        I: AsRef<str>,
        M: AsRef<str>,
        F: FnOnce(BufReader<PipeReader>, BufWriter<File>) + Send + 'static,
    {
        let name = name.as_ref();
        let log_file_path = {
            let lock = LOG_DIR.read().expect("Should lock LOG_DIR");
            lock.join(format!("{}.log", name.replace(['/', ':', '.'], "_")))
        };

        let message = message.as_ref();
        let (reader, writer) = os_pipe::pipe()?;
        let name_color = color_str(name, color);

        self.stdout(writer.try_clone()?)
            .stderr(writer)
            .stdin(Stdio::piped());

        let progress = Logger::multi_progress()
            .add(ProgressBar::new_spinner().with_message(format!("{message} {name_color}")));
        progress.enable_steady_tick(Duration::from_millis(100));

        let mut child = self.spawn()?;

        let child_pid = child.id();
        add_pid(child_pid);

        // We drop the `Command` to prevent blocking on writer
        // https://docs.rs/os_pipe/latest/os_pipe/#examples
        drop(self);

        let reader = BufReader::new(reader);
        let log_file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_path.as_path())?,
        );

        thread::spawn(move || log_handler(reader, log_file));

        let status = child.wait()?;
        remove_pid(child_pid);

        progress.finish();
        Logger::multi_progress().remove(&progress);

        Ok(status)
    }
}

/// Used to keep the style of logs consistent between
/// normal log use and command output.
fn log_header<T>(text: T) -> String
where
    T: AsRef<str>,
{
    fn inner(text: &str) -> String {
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
    inner(text.as_ref())
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
fn shorten_name<T>(text: T) -> String
where
    T: AsRef<str>,
{
    let text = text.as_ref();

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

#[must_use]
pub fn gen_random_ansi_color() -> u8 {
    // ANSI extended color range
    // https://www.ditig.com/publications/256-colors-cheat-sheet
    const LOW_END: u8 = 21; // Blue1 #0000ff rgb(0,0,255) hsl(240,100%,50%)
    const HIGH_END: u8 = 230; // Cornsilk1 #ffffd7 rgb(255,255,215) hsl(60,100%,92%)

    rand::thread_rng().gen_range(LOW_END..=HIGH_END)
}

pub fn color_str<T>(text: T, ansi_color: u8) -> String
where
    T: AsRef<str>,
{
    if ShouldColorize::from_env().should_colorize() {
        Color::Fixed(ansi_color)
            .paint(text.as_ref().to_string())
            .to_string()
    } else {
        text.as_ref().to_string()
    }
}
