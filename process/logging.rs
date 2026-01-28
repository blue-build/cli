use std::{
    borrow::Cow,
    fs::OpenOptions,
    io::{BufRead, BufReader, Result, Write as IoWrite},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use blue_build_utils::get_env_var;
use bon::Builder;
use chrono::Local;
use colored::{ColoredString, Colorize, control::ShouldColorize};
use indicatif::{MultiProgress, ProgressBar};
use indicatif_log_bridge::LogWrapper;
use log::{Level, LevelFilter, Record, warn};
use log4rs::{
    Config, Logger as L4RSLogger,
    append::{
        console::ConsoleAppender,
        rolling_file::{
            RollingFileAppender,
            policy::compound::{
                CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger,
            },
        },
    },
    config::{Appender, Root},
    encode::{Encode, Write, pattern::PatternEncoder},
};
use nu_ansi_term::Color;
use private::Private;
use rand::seq::SliceRandom;

use crate::signal_handler::{add_pid, remove_pid};

mod private {
    pub trait Private {}
}

impl Private for Command {}

static MULTI_PROGRESS: std::sync::LazyLock<MultiProgress> =
    std::sync::LazyLock::new(MultiProgress::new);
static LOG_DIR: std::sync::LazyLock<Mutex<PathBuf>> =
    std::sync::LazyLock::new(|| Mutex::new(PathBuf::new()));

#[derive(Debug, Clone)]
pub struct Logger {
    modules: Vec<(String, LevelFilter)>,
    level: LevelFilter,
    log_dir: Option<PathBuf>,
}

impl Logger {
    const TRIGGER_FILE_SIZE: u64 = 10 * 1024;
    const ARCHIVE_FILENAME_PATTERN: &'static str = "bluebuild.{}.log";
    const LOG_FILENAME: &'static str = "bluebuild.log";
    const LOG_FILE_COUNT: u32 = 4;

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

    pub const fn filter_level(&mut self, filter_level: LevelFilter) -> &mut Self {
        self.level = filter_level;
        self
    }

    pub fn log_out_dir<P>(&mut self, path: Option<P>) -> &mut Self
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
    pub fn init(&self) {
        let home = get_env_var("HOME").expect("$HOME should be defined");
        let log_dir = self.log_dir.as_ref().map_or_else(
            || Path::new(home.as_str()).join(".cache/bluebuild"),
            Clone::clone,
        );

        let mut lock = LOG_DIR.lock().expect("Should lock LOG_DIR");
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

        let config =
            Config::builder().appender(Appender::builder().build("stderr", Box::new(stderr)));
        let mut root = Root::builder().appender("stderr");

        let config = {
            let file_appender = FixedWindowRoller::builder()
                .build(&log_archive_pattern, Self::LOG_FILE_COUNT)
                .and_then(|window_roller| {
                    Ok(RollingFileAppender::builder()
                        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}{n}")))
                        .build(
                            log_out_path,
                            Box::new(CompoundPolicy::new(
                                Box::new(SizeTrigger::new(Self::TRIGGER_FILE_SIZE)),
                                Box::new(window_roller),
                            )),
                        )?)
                });
            match file_appender {
                Err(e) => {
                    eprintln!("Cannot create logs directory:\n{e}");
                    config
                }
                Ok(file_appender) => {
                    root = root.appender("file");
                    config.appender(Appender::builder().build("file", Box::new(file_appender)))
                }
            }
            .build(root.build(self.level))
            .expect("Logger config should build")
        };

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

pub trait CommandLogging: Private {
    /// Prints each line of stdout/stderr with an image ref string
    /// and a progress spinner while also logging the build output.
    /// This helps to keep track of every build running in parallel.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn build_status<T, U>(self, image_ref: T, message: U) -> Result<ExitStatus>
    where
        T: AsRef<str>,
        U: AsRef<str>;

    /// Prints each line of stdout/stderr with a log header
    /// and a progress spinner. This helps to keep track of every
    /// command running in parallel.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn message_status<S, D>(self, header: S, message: D) -> Result<ExitStatus>
    where
        S: AsRef<str>,
        D: Into<Cow<'static, str>>;
}

impl CommandLogging for Command {
    fn build_status<T, U>(self, image_ref: T, message: U) -> Result<ExitStatus>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        fn inner(mut command: Command, image_ref: &str, message: &str) -> Result<ExitStatus> {
            let ansi_color = gen_random_ansi_color();
            let name = color_str(image_ref, ansi_color);
            let short_name = color_str(shorten_name(image_ref), ansi_color);
            let (reader, writer) = os_pipe::pipe()?;

            command
                .stdout(writer.try_clone()?)
                .stderr(writer)
                .stdin(Stdio::piped());

            let progress = Logger::multi_progress()
                .add(ProgressBar::new_spinner().with_message(format!("{message} {name}")));
            progress.enable_steady_tick(Duration::from_millis(100));

            let mut child = command.spawn()?;

            let child_pid = child.id();
            add_pid(child_pid);

            // We drop the `Command` to prevent blocking on writer
            // https://docs.rs/os_pipe/latest/os_pipe/#examples
            drop(command);

            let reader = BufReader::new(reader);
            let log_file_path = {
                let lock = LOG_DIR.lock().expect("Should lock LOG_DIR");
                lock.join(format!("{}.log", image_ref.replace(['/', ':', '.'], "_")))
            };
            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_path.as_path())?;

            thread::spawn(move || {
                let mp = Logger::multi_progress();
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        let text =
                            format!("{log_prefix} {l}", log_prefix = log_header(&short_name));
                        if mp.is_hidden() {
                            eprintln!("{text}");
                        } else {
                            mp.println(text).unwrap();
                        }
                        if let Err(e) = writeln!(&log_file, "{l}") {
                            warn!(
                                "Failed to write to log for build {}: {e:?}",
                                log_file_path.display()
                            );
                        }
                    }
                });
            });

            let status = child.wait()?;
            remove_pid(child_pid);

            progress.finish();
            Logger::multi_progress().remove(&progress);

            Ok(status)
        }
        inner(self, image_ref.as_ref(), message.as_ref())
    }

    fn message_status<S, D>(self, header: S, message: D) -> Result<ExitStatus>
    where
        S: AsRef<str>,
        D: Into<Cow<'static, str>>,
    {
        fn inner(
            mut command: Command,
            header: &str,
            message: Cow<'static, str>,
        ) -> Result<ExitStatus> {
            let ansi_color = gen_random_ansi_color();
            let header = color_str(header, ansi_color);
            let (reader, writer) = os_pipe::pipe()?;

            command
                .stdout(writer.try_clone()?)
                .stderr(writer)
                .stdin(Stdio::piped());

            let progress =
                Logger::multi_progress().add(ProgressBar::new_spinner().with_message(message));
            progress.enable_steady_tick(Duration::from_millis(100));

            let mut child = command.spawn()?;

            let child_pid = child.id();
            add_pid(child_pid);

            // We drop the `Command` to prevent blocking on writer
            // https://docs.rs/os_pipe/latest/os_pipe/#examples
            drop(command);

            let reader = BufReader::new(reader);

            thread::spawn(move || {
                let mp = Logger::multi_progress();
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        let text = format!("{log_prefix} {l}", log_prefix = log_header(&header));
                        if mp.is_hidden() {
                            eprintln!("{text}");
                        } else {
                            mp.println(text).unwrap();
                        }
                    }
                });
            });

            let status = child.wait()?;
            remove_pid(child_pid);

            progress.finish();
            Logger::multi_progress().remove(&progress);

            Ok(status)
        }
        inner(self, header.as_ref(), message.into())
    }
}

#[derive(Debug, Builder)]
struct CustomPatternEncoder {
    #[builder(default, into)]
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
                        module = record.module_path().unwrap_or("").bright_yellow(),
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

    let Some(path) = parts.next() else {
        return text.to_string();
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

// ANSI extended color range:
// https://www.ditig.com/publications/256-colors-cheat-sheet
//
// The following ANSI color codes are exactly the color codes that have a contrast ratio of
// at least 4.0 on both white and black backgrounds, as defined by WCAG 2.2:
// https://www.w3.org/TR/WCAG22/#dfn-contrast-ratio
// This ensures that the colors are legible in both light and dark mode.
// (WCAG 2.2 requires a contrast ratio of 4.5 for accessibility, but there are too few colors
// that meet that requirement on both white and black backgrounds.)
const MID_COLORS: [u8; 22] = [
    27, 28, 29, 30, 31, 62, 63, 64, 65, 96, 97, 98, 99, 129, 130, 131, 132, 133, 161, 162, 163, 164,
];

/// Generate random ANSI colors that are legible on both light and dark backgrounds.
///
/// More precisely, all generated colors have a contrast ratio of at least 4.0 (as defined by
/// WCAG 2.2) on both white and black backgrounds.
///
/// This function internally keeps track of state and will cycle through all such colors in a
/// random order before repeating colors.
#[must_use]
pub fn gen_random_ansi_color() -> u8 {
    static SHUFFLED_COLORS: LazyLock<[u8; MID_COLORS.len()]> = LazyLock::new(|| {
        let mut colors = MID_COLORS;
        colors.shuffle(&mut rand::rng());
        colors
    });
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = COUNTER.fetch_add(1, Ordering::Relaxed) % MID_COLORS.len();
    SHUFFLED_COLORS[index]
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
