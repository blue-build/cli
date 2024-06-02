use std::{
    env,
    io::{BufRead, BufReader, Result},
    process::{Command, ExitStatus},
    sync::Arc,
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
use rand::Rng;

static MULTI_PROGRESS: Lazy<MultiProgress> = Lazy::new(MultiProgress::new);

pub struct Logger {
    modules: Vec<(String, LevelFilter)>,
    level: LevelFilter,
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
        let home = env::var("HOME").expect("$HOME should be defined");
        let log_dir = format!("{home}/.local/share/bluebuild");
        let log_out_path = format!("{log_dir}/{}", Self::LOG_FILENAME);
        let log_archive_pattern = format!("{log_dir}/{}", Self::ARCHIVE_FILENAME_PATTERN);

        let stderr = ConsoleAppender::builder()
            .encoder(Box::new(CustomPatternEncoder))
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
    /// Prints each line of stdout/stderr with an image ref string
    /// and a progress spinner. This helps to keep track of every
    /// build running in parallel.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn status_image_ref_progress<T, U>(self, image_ref: T, message: U) -> Result<ExitStatus>
    where
        T: AsRef<str>,
        U: AsRef<str>;
}

impl CommandLogging for Command {
    fn status_image_ref_progress<T, U>(mut self, image_ref: T, message: U) -> Result<ExitStatus>
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        let ansi_color = gen_random_ansi_color();
        let name = color_str(&image_ref, ansi_color);
        let short_name = color_str(shorten_name(&image_ref), ansi_color);
        let log_prefix = Arc::new(log_header(short_name));
        let (reader, writer) = os_pipe::pipe()?;

        self.stdout(writer.try_clone()?).stderr(writer);

        let progress = Logger::multi_progress()
            .add(ProgressBar::new_spinner().with_message(format!("{} {name}", message.as_ref())));
        progress.enable_steady_tick(Duration::from_millis(100));

        let mut child = self.spawn()?;

        // We drop the `Command` to prevent blocking on writer
        // https://docs.rs/os_pipe/latest/os_pipe/#examples
        drop(self);

        let reader = BufReader::new(reader);

        thread::spawn(move || {
            let mp = Logger::multi_progress();
            reader.lines().for_each(|line| {
                if let Ok(l) = line {
                    let text = format!("{log_prefix} {l}");
                    if mp.is_hidden() {
                        eprintln!("{text}");
                    } else {
                        mp.println(text).unwrap();
                    }
                }
            });
        });

        let status = child.wait()?;

        progress.finish();
        Logger::multi_progress().remove(&progress);

        Ok(status)
    }
}

#[derive(Debug)]
struct CustomPatternEncoder;

impl Encode for CustomPatternEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
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

fn gen_random_ansi_color() -> u8 {
    // ANSI extended color range
    // https://www.ditig.com/publications/256-colors-cheat-sheet
    const LOW_END: u8 = 21; // Blue1 #0000ff rgb(0,0,255) hsl(240,100%,50%)
    const HIGH_END: u8 = 230; // Cornsilk1 #ffffd7 rgb(255,255,215) hsl(60,100%,92%)

    rand::thread_rng().gen_range(LOW_END..=HIGH_END)
}

fn color_str<T>(text: T, ansi_color: u8) -> String
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
