use std::io::{self, Write};

use chrono::Local;
use colored::{ColoredString, Colorize};
use env_logger::fmt::Formatter;
use log::{Level, LevelFilter, Record};

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

/// Given a `LevelFilter`, returns the function
/// used to format logs. The more verbose the log level,
/// the more info is displayed in each log header.
pub fn format_log(
    log_level: LevelFilter,
) -> impl Fn(&mut Formatter, &Record) -> io::Result<()> + Sync + Send {
    move |buf: &mut Formatter, record: &Record| match log_level {
        LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => {
            writeln!(
                buf,
                "{level:width$} {sep} {args}",
                level = record.level().colored(),
                width = 5,
                sep = "=>".bold(),
                args = record.args(),
            )
        }
        LevelFilter::Debug => writeln!(
            buf,
            "[{time} {level:>width$}] {sep} {args}",
            time = Local::now().format("%H:%M:%S"),
            level = record.level().colored(),
            sep = "=>".bold(),
            args = record.args(),
            width = 5,
        ),
        LevelFilter::Trace => writeln!(
            buf,
            "[{time} {level:width$} {module}:{line}] {sep} {args}",
            time = Local::now().format("%H:%M:%S"),
            level = record.level().colored(),
            module = record
                .module_path()
                .map_or_else(|| "", |p| p)
                .bright_yellow(),
            line = record
                .line()
                .map_or_else(String::new, |l| l.to_string())
                .bright_green(),
            sep = "=>".bold(),
            args = record.args(),
            width = 5,
        ),
        LevelFilter::Off => Ok(()),
    }
}
