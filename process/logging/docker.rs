#![expect(dead_code)]

use std::{
    borrow::Cow,
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    process::Command,
};

use base64::prelude::*;
use bon::{bon, Builder};
use chrono::{DateTime, Utc};
use log::trace;
use miette::{bail, Context, IntoDiagnostic, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer};

use super::{color_str, gen_random_ansi_color, shorten_name, Logger, ProgressLogger};

#[derive(Debug, Deserialize)]
struct JsonLogLine {
    vertexes: Option<Vec<VertexEntry>>,
    statuses: Option<Vec<StatusEntry>>,
    logs: Option<Vec<LogEntry>>,
}

#[derive(Debug, Deserialize)]
struct VertexEntry {
    digest: String,
    name: String,
    started: Option<DateTime<Utc>>,
}

static STAGE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([\w_-]+)\s+(\d+)/(\d+)\]\s+.*").unwrap());

impl<'a> VertexEntry {
    pub fn stage_name(&'a self) -> Cow<'a, str> {
        STAGE.replace_all(&self.name, "$1")
    }
}

#[derive(Debug, Deserialize)]
struct StatusEntry {
    id: String,
    vertex: String,
    current: usize,
    timestamp: DateTime<Utc>,
    started: DateTime<Utc>,
    name: Option<String>,
    total: Option<usize>,
    completed: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct LogEntry {
    vertex: String,
    stream: usize,

    #[serde(deserialize_with = "deser_base64")]
    data: String,
    timestamp: DateTime<Utc>,
}

fn deser_base64<'de, D>(deserializer: D) -> std::prelude::v1::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let out = BASE64_STANDARD
        .decode(&value)
        .into_diagnostic()
        .with_context(|| format!("Unable to decode log entry {value}"))
        .map_err(serde::de::Error::custom)?;
    String::from_utf8(out).map_err(serde::de::Error::custom)
}

#[bon]
impl LogEntry {
    #[builder]
    pub fn print_log(&self, color: u8, vertex: &VertexEntry) -> String {
        format!("{} {}", color_str(vertex.stage_name(), color), self.data)
    }
}

#[derive(Debug, Builder)]
#[builder(on(String, into))]
struct JsonLog<R, W>
where
    R: Read,
    W: Write,
{
    log_reader: BufReader<R>,
    log_file_writer: BufWriter<W>,
    color: u8,
    name: String,
    short_name: String,

    #[builder(default)]
    vertex_map: HashMap<String, VertexEntry>,
}

impl<R, W> JsonLog<R, W>
where
    R: Read,
    W: Write,
{
    pub fn print_log(mut self) {
        let mp = Logger::multi_progress();

        for line in self.log_reader.lines().map_while(Result::ok) {
            if let Ok(mut log_line) = serde_json::from_str::<JsonLogLine>(&line) {
                trace!("{log_line:?}");

                if let Some(vertexes) = log_line.vertexes.take() {
                    for vertex in vertexes {
                        let _ = self.vertex_map.insert(vertex.digest.clone(), vertex);
                    }
                }

                if let Some(logs) = log_line.logs.take() {
                    for log in logs {
                        let vertex = self.vertex_map.get(&log.vertex).unwrap();
                        let out = log.print_log().color(self.color).vertex(vertex).call();

                        if mp.is_hidden() {
                            eprintln!("{out}");
                        } else {
                            mp.println(out).unwrap();
                        }
                    }
                }
            }

            if let Err(e) = writeln!(self.log_file_writer, "{line}") {
                trace!(
                    "!! Failed to write to log for build {}:\n{e:?}",
                    self.short_name
                );
            }
        }

        if let Err(e) = self.log_file_writer.flush() {
            trace!("!! Failed to flush log file:\n{e:?}");
        }
    }
}

#[allow(private_bounds)]
pub trait DockerLogging: ProgressLogger {
    /// Prints the output of plaintext docker build logs
    /// using custom progress bars and headers.
    ///
    /// # Errors
    /// Will error if the child process cannot spawn,
    /// or the docker build fails. The build error will
    /// be packaged in a `miette::Report` for easier display.
    fn docker_log<I>(self, image_ref: I) -> Result<()>
    where
        I: AsRef<str>;
}

impl DockerLogging for Command {
    fn docker_log<I>(self, image_ref: I) -> Result<()>
    where
        I: AsRef<str>,
    {
        let image_ref = image_ref.as_ref();
        let ansi_color = gen_random_ansi_color();
        let short_name = color_str(shorten_name(image_ref), ansi_color);

        let status = self
            .progress_log(ansi_color, image_ref, "Building image", {
                let image_ref = image_ref.to_string();
                move |reader, log_file| {
                    JsonLog::builder()
                        .log_reader(reader)
                        .log_file_writer(log_file)
                        .color(ansi_color)
                        .name(&image_ref)
                        .short_name(short_name)
                        .build()
                        .print_log();
                }
            })
            .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to build image {}", color_str(image_ref, ansi_color));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::{BufReader, BufWriter, Cursor};

    use crate::logging::{color_str, gen_random_ansi_color, shorten_name};

    use super::{JsonLog, JsonLogLine};

    const DOCKER_LOG: &str = include_str!("../../test-files/docker-logs/dockerout.json");

    #[test]
    fn read_docker_log() {
        for line in DOCKER_LOG.lines() {
            eprintln!("{line}");
            let line: JsonLogLine = serde_json::from_str(line).unwrap();
            dbg!(line);
        }
    }

    #[test]
    fn process_log() {
        let reader = BufReader::new(Cursor::new(DOCKER_LOG));
        let mut out_file = tempfile::NamedTempFile::new().unwrap();
        let writer = BufWriter::new(out_file.as_file_mut());
        let image_ref = "ghcr.io/blue-build/cli/test";
        let ansi_color = gen_random_ansi_color();
        let short_name = color_str(shorten_name(image_ref), ansi_color);

        JsonLog::builder()
            .log_reader(reader)
            .log_file_writer(writer)
            .color(ansi_color)
            .name(image_ref)
            .short_name(short_name)
            .build()
            .print_log();
    }
}
