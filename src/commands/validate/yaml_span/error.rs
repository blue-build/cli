use miette::Diagnostic;
use thiserror::Error;
use yaml_rust2::{Event, ScanError};

#[derive(Error, Diagnostic, Debug)]
pub enum YamlSpanError {
    #[error("Failed to parse file: {0}")]
    #[diagnostic()]
    ScanError(#[from] ScanError),

    #[error("Failed to read event: {0:?}")]
    #[diagnostic()]
    UnexpectedEvent(Event),

    #[error("Encountered key {key} when looking for index {index}")]
    #[diagnostic()]
    ExpectIndexFoundKey { key: String, index: usize },

    #[error("Reached end of map an haven't found key {0}")]
    #[diagnostic()]
    EndOfMapNoKey(String),

    #[error("Reached end of sequence before reaching index {0}")]
    #[diagnostic()]
    EndOfSequenceNoIndex(usize),

    #[error("Encountered scalar value {value} when looking for {segment}")]
    #[diagnostic()]
    UnexpectedScalar { value: String, segment: String },
}
