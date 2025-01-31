use std::{path::PathBuf, sync::Arc};

use colored::Colorize;
use miette::{Diagnostic, LabeledSpan, NamedSource};
use thiserror::Error;

use crate::commands::validate::yaml_span::YamlSpanError;

#[derive(Error, Diagnostic, Debug)]
pub enum SchemaValidateBuilderError {
    #[error(transparent)]
    #[cfg(not(test))]
    #[diagnostic()]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    #[cfg(test)]
    #[diagnostic()]
    Fs(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic()]
    JsonSchemaBuild(#[from] jsonschema::ValidationError<'static>),
}

#[derive(Error, Diagnostic, Debug)]
pub enum SchemaValidateError {
    #[error("Failed to deserialize file {}", .1.display().to_string().bold().italic())]
    #[diagnostic()]
    SerdeYaml(serde_yaml::Error, PathBuf),

    #[error(
        "{} error{} encountered",
        .labels.len().to_string().red(),
        if .labels.len() == 1 { "" } else { "s" }
    )]
    #[diagnostic()]
    YamlValidate {
        #[source_code]
        src: NamedSource<Arc<String>>,

        #[label(collection)]
        labels: Vec<LabeledSpan>,

        #[help]
        help: String,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    YamlSpan(#[from] YamlSpanError),
}
