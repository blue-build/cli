use std::io::{self, Write};

use chrono::Local;
use colored::{ColoredString, Colorize};
use env_logger::fmt::Formatter;
use log::{Level, LevelFilter, Record};

fn colored_level(level: Level) -> ColoredString {
    match level {
        Level::Error => Level::Error.as_str().red(),
        Level::Warn => Level::Warn.as_str().yellow(),
        Level::Info => Level::Info.as_str().green(),
        Level::Debug => Level::Debug.as_str().blue(),
        Level::Trace => Level::Trace.as_str().cyan(),
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
                "{:width$} {} {}",
                colored_level(record.level()),
                "=>".bold(),
                record.args(),
                width = 5,
            )
        }
        LevelFilter::Debug => writeln!(
            buf,
            "[{} {:>width$}] {} {}",
            Local::now().format("%H:%M:%S"),
            colored_level(record.level()),
            "=>".bold(),
            record.args(),
            width = 5,
        ),
        LevelFilter::Trace => writeln!(
            buf,
            "[{} {:width$} {}:{}] {} {}",
            Local::now().format("%H:%M:%S"),
            colored_level(record.level()),
            record
                .module_path()
                .map_or_else(|| "", |p| p)
                .bright_yellow(),
            record
                .line()
                .map_or_else(String::new, |l| l.to_string())
                .bright_green(),
            "=>".bold(),
            record.args(),
            width = 5,
        ),
        LevelFilter::Off => Ok(()),
    }
}
