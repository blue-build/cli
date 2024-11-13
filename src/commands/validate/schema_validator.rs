use std::{
    borrow::Cow,
    collections::HashSet,
    path::Path,
    sync::{Arc, LazyLock},
};

use blue_build_process_management::ASYNC_RUNTIME;
use bon::bon;
use cached::proc_macro::cached;
use colored::Colorize;
use indexmap::IndexMap;
use jsonschema::{
    output::Output, BasicOutput, ErrorIterator, Retrieve, Uri, ValidationError, Validator,
};
use log::{debug, trace};
use miette::{bail, miette, Context, IntoDiagnostic, LabeledSpan, NamedSource, Report, Result};
use regex::Regex;
use serde_json::Value;

use super::{location::Location, yaml_span::YamlSpan};

pub const BASE_SCHEMA_URL: &str = "https://schema.blue-build.org";
pub const RECIPE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/recipe-v1.json";
pub const STAGE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/stage-v1.json";
pub const MODULE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/module-v1.json";
pub const MODULE_STAGE_LIST_V1_SCHEMA_URL: &str =
    "https://schema.blue-build.org/module-stage-list-v1.json";

#[derive(Debug, Clone)]
pub struct SchemaValidator {
    schema: Arc<Value>,
    validator: Arc<Validator>,
    url: &'static str,
}

#[bon]
impl SchemaValidator {
    #[builder]
    pub async fn new(url: &'static str) -> Result<Self, Report> {
        tokio::spawn(async move {
            let schema: Arc<Value> = Arc::new(
                reqwest::get(url)
                    .await
                    .into_diagnostic()
                    .with_context(|| format!("Failed to get schema at {url}"))?
                    .json()
                    .await
                    .into_diagnostic()
                    .with_context(|| format!("Failed to get json for schema {url}"))?,
            );
            let validator = Arc::new(
                tokio::task::spawn_blocking({
                    let schema = schema.clone();
                    move || {
                        jsonschema::options()
                            .with_retriever(ModuleSchemaRetriever)
                            .build(&schema)
                            .into_diagnostic()
                            .with_context(|| format!("Failed to build validator for schema {url}"))
                    }
                })
                .await
                .expect("Should join blocking thread")?,
            );

            Ok(Self {
                schema,
                validator,
                url,
            })
        })
        .await
        .expect("Should join task")
    }

    pub fn apply<'a, 'b>(&'a self, value: &'b Value) -> Output<'a, 'b> {
        self.validator.apply(value)
    }

    pub fn iter_errors<'a>(&'a self, value: &'a Value) -> ErrorIterator<'a> {
        self.validator.iter_errors(value)
    }

    pub fn schema(&self) -> Arc<Value> {
        self.schema.clone()
    }

    pub const fn url(&self) -> &'static str {
        self.url
    }

    pub fn process_validation(
        &self,
        path: &Path,
        file: Arc<String>,
        all_errors: bool,
    ) -> Result<Option<Report>> {
        let recipe_path_display = path.display().to_string().bold().italic();

        let spanner = YamlSpan::builder().file(file.clone()).build()?;
        let instance: Value = serde_yaml::from_str(&file)
            .into_diagnostic()
            .with_context(|| format!("Failed to deserialize recipe {recipe_path_display}"))?;
        trace!("{recipe_path_display}:\n{file}");

        Ok(if all_errors {
            self.process_basic_output(self.apply(&instance).basic(), file, &spanner, path)
        } else {
            self.process_err(self.iter_errors(&instance), path, file, &spanner)
        })
    }

    fn process_basic_output(
        &self,
        out: BasicOutput<'_>,
        file: Arc<String>,
        spanner: &YamlSpan,
        path: &Path,
    ) -> Option<Report> {
        match out {
            BasicOutput::Valid(_) => None,
            BasicOutput::Invalid(errors) => {
                let mut collection: IndexMap<Location, Vec<String>> = IndexMap::new();
                let errors = {
                    let mut e = errors.into_iter().collect::<Vec<_>>();
                    e.sort_by(|e1, e2| {
                        e1.instance_location()
                            .as_str()
                            .cmp(e2.instance_location().as_str())
                    });
                    e
                };
                let errors: Vec<(Location, String)> = {
                    let e = errors
                        .into_iter()
                        .map(|e| {
                            (
                                Location::from(e.instance_location()),
                                remove_json(&e.error_description().to_string()).to_string(),
                            )
                        })
                        .collect::<HashSet<_>>();
                    let mut e = e.into_iter().collect::<Vec<_>>();
                    e.sort_by(|e1, e2| e1.0.as_str().cmp(e2.0.as_str()));
                    e
                };

                for (instance_path, err) in errors {
                    collection
                        .entry(instance_path)
                        .and_modify(|errs| {
                            errs.push(format!("- {}", err.bold().red()));
                        })
                        .or_insert_with(|| vec![format!("- {}", err.bold().red())]);
                }

                let spans = collection
                    .into_iter()
                    .map(|(key, value)| {
                        LabeledSpan::new_with_span(
                            Some(value.join("\n")),
                            spanner.get_span(&key).unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                Some(
                    miette!(
                        labels = spans,
                        help = format!(
                            "Try adding these lines to the top of your file:\n{}\n{}",
                            "---".bright_green(),
                            format!("# yaml-language-server: $schema={}", self.url).bright_green(),
                        ),
                        "{} error{} encountered",
                        spans.len().to_string().red(),
                        if spans.len() == 1 { "" } else { "s" }
                    )
                    .with_source_code(
                        NamedSource::new(path.display().to_string(), file).with_language("yaml"),
                    ),
                )
            }
        }
    }

    fn process_err<'a, I>(
        &self,
        errors: I,
        path: &Path,
        file: Arc<String>,
        spanner: &YamlSpan,
    ) -> Option<Report>
    where
        I: Iterator<Item = ValidationError<'a>>,
    {
        let spans = errors
            .map(|err| {
                LabeledSpan::new_primary_with_span(
                    Some(remove_json(&err.to_string()).bold().red().to_string()),
                    spanner
                        .get_span(&Location::from(err.instance_path))
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();

        if spans.is_empty() {
            None
        } else {
            Some(
                miette!(
                    labels = spans,
                    help = format!(
                        "Try adding these lines to the top of your file:\n{}\n{}",
                        "---".bright_green(),
                        format!("# yaml-language-server: $schema={}", self.url).bright_green(),
                    ),
                    "{} error{} encountered",
                    spans.len().to_string().red(),
                    if spans.len() == 1 { "" } else { "s" }
                )
                .with_source_code(
                    NamedSource::new(path.display().to_string(), file).with_language("yaml"),
                ),
            )
        }
    }
}

fn remove_json(string: &str) -> Cow<'_, str> {
    static REGEX_OBJECT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\{.*\}\s(.*)$").unwrap());
    static REGEX_ARRAY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[.*\]\s(.*)$").unwrap());

    let string = string.trim();

    if REGEX_OBJECT.is_match(string) {
        REGEX_OBJECT.replace_all(string, "$1")
    } else if REGEX_ARRAY.is_match(string) {
        REGEX_ARRAY.replace_all(string, "$1")
    } else {
        Cow::Borrowed(string)
    }
}

struct ModuleSchemaRetriever;

impl Retrieve for ModuleSchemaRetriever {
    fn retrieve(
        &self,
        uri: &Uri<&str>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ASYNC_RUNTIME.block_on(cache_retrieve(uri))?)
    }
}

#[cached(result = true, key = "String", convert = r#"{ format!("{uri}") }"#)]
async fn cache_retrieve(uri: &Uri<&str>) -> miette::Result<Value> {
    let scheme = uri.scheme();
    let path = uri.path();

    let uri = match scheme.as_str() {
        "json-schema" => {
            format!("{BASE_SCHEMA_URL}{path}")
        }
        "https" => uri.to_string(),
        scheme => bail!("Unknown scheme {scheme}"),
    };

    debug!("Retrieving schema from {}", uri.bold().italic());
    tokio::spawn(async move {
        reqwest::get(&uri)
            .await
            .into_diagnostic()
            .with_context(|| format!("Failed to retrieve schema from {uri}"))?
            .json()
            .await
            .into_diagnostic()
            .with_context(|| format!("Failed to parse json from {uri}"))
            .inspect(|value| trace!("{}:\n{value}", uri.bold().italic()))
    })
    .await
    .expect("Should join task")
}
