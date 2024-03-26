use std::io::{self, Write};

use chrono::Local;
use colored::{ColoredString, Colorize};
use env_logger::fmt::Formatter;
use log::{Level, LevelFilter, Record};

fn colored_level(level: Level) -> ColoredString {
    match level {
        Level::Error => Level::Error.as_str().bright_red(),
        Level::Warn => Level::Warn.as_str().yellow(),
        Level::Info => Level::Info.as_str().bright_green(),
        Level::Debug => Level::Debug.as_str().blue(),
        Level::Trace => Level::Trace.as_str().bright_cyan(),
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
                "{} => {}",
                colored_level(record.level()),
                record.args()
            )
        }
        LevelFilter::Debug => writeln!(
            buf,
            "[{} {}] => {}",
            Local::now().format("%H:%M:%S"),
            colored_level(record.level()),
            record.args(),
        ),
        LevelFilter::Trace => writeln!(
            buf,
            "[{} {} {}:{}] => {}",
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
            record.args(),
        ),
        LevelFilter::Off => Ok(()),
    }
}
