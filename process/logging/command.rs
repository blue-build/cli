use std::{
    io::{BufRead, Result, Write},
    process::{Command, ExitStatus},
};

use log::trace;

use super::{color_str, gen_random_ansi_color, log_header, shorten_name, Logger, ProgressLogger};

#[allow(private_bounds)]
pub trait CommandLogging: ProgressLogger {
    /// Prints each line of stdout/stderr with an image ref string
    /// and a progress spinner. This helps to keep track of every
    /// build running in parallel.
    ///
    /// # Errors
    /// Will error if there was an issue executing the process.
    fn status_image_ref_progress<I, M>(self, image_ref: I, message: M) -> Result<ExitStatus>
    where
        I: AsRef<str>,
        M: AsRef<str>;
}

impl CommandLogging for Command {
    fn status_image_ref_progress<I, M>(self, image_ref: I, message: M) -> Result<ExitStatus>
    where
        I: AsRef<str>,
        M: AsRef<str>,
    {
        let ansi_color = gen_random_ansi_color();
        let short_name = color_str(shorten_name(&image_ref), ansi_color);
        let mp = Logger::multi_progress();

        self.progress_log(
            ansi_color,
            image_ref,
            message,
            move |reader, mut log_file| {
                reader.lines().for_each(|line| {
                    if let Ok(l) = line {
                        if !l.is_empty() {
                            let text =
                                format!("{log_prefix} {l}", log_prefix = log_header(&short_name));
                            if mp.is_hidden() {
                                eprintln!("{text}");
                            } else {
                                mp.println(text).unwrap();
                            }
                            if let Err(e) = writeln!(log_file, "{l}") {
                                trace!("!! Failed to write to log for build {short_name}:\n{e:?}");
                            }
                        }
                    }
                });
                if let Err(e) = log_file.flush() {
                    trace!("!! Failed to flush log file:\n{e:?}");
                }
            },
        )
    }
}
