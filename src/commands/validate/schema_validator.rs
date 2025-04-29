use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, LazyLock},
};

use blue_build_process_management::ASYNC_RUNTIME;
use blue_build_recipe::ModuleTypeVersion;
use blue_build_utils::constants::{
    CUSTOM_MODULE_SCHEMA, IMPORT_MODULE_SCHEMA, JSON_SCHEMA, STAGE_SCHEMA,
};
use bon::bon;
use cached::proc_macro::cached;
use colored::Colorize;
use indexmap::IndexMap;
use jsonschema::{BasicOutput, Retrieve, Uri, ValidationError, Validator};
use miette::{Context, IntoDiagnostic, LabeledSpan, NamedSource};
use regex::Regex;
use serde_json::Value;

use super::{location::Location, yaml_span::YamlSpan};

#[cfg(test)]
use std::eprintln as trace;
#[cfg(test)]
use std::eprintln as warn;

#[cfg(not(test))]
use log::{trace, warn};

mod error;

pub use error::*;

#[derive(Debug, Clone)]
pub struct SchemaValidator {
    validator: Arc<Validator>,
    url: &'static str,
    all_errors: bool,
}

#[bon]
impl SchemaValidator {
    #[builder]
    pub async fn new(
        /// The URL of the schema to validate against
        url: &'static str,
        /// Produce all errors found
        #[builder(default)]
        all_errors: bool,
    ) -> Result<Self, SchemaValidateBuilderError> {
        tokio::spawn(async move {
            let schema: Value = {
                #[cfg(not(test))]
                {
                    reqwest::get(url).await?.json().await?
                }
                #[cfg(test)]
                {
                    serde_json::from_slice(std::fs::read_to_string(url)?.as_bytes())?
                }
            };
            let validator = Arc::new(
                tokio::task::spawn_blocking({
                    let schema = schema.clone();
                    move || {
                        jsonschema::options()
                            .with_retriever(ModuleSchemaRetriever)
                            .build(&schema)
                    }
                })
                .await
                .expect("Should join blocking thread")?,
            );

            Ok(Self {
                validator,
                url,
                all_errors,
            })
        })
        .await
        .expect("Should join task")
    }

    pub fn process_validation<P>(
        &self,
        path: P,
        file: Arc<String>,
    ) -> Result<(), SchemaValidateError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let spans = self.get_spans(&file, path)?;

        self.spans_to_report(spans, file, path)
    }

    fn get_spans(
        &self,
        file: &Arc<String>,
        path: &Path,
    ) -> Result<Vec<LabeledSpan>, SchemaValidateError> {
        let recipe_path_display = path.display().to_string().bold().italic();
        let spanner = YamlSpan::builder().file(file.clone()).build()?;
        let instance: Value = serde_yaml::from_str(file)
            .map_err(|e| SchemaValidateError::SerdeYaml(e, path.to_path_buf()))?;
        trace!("{recipe_path_display}:\n{file}");

        Ok(if self.all_errors {
            process_basic_output(self.validator.apply(&instance).basic(), &spanner)
        } else {
            process_err(self.validator.iter_errors(&instance), &spanner)
        })
    }

    fn spans_to_report(
        &self,
        labels: Vec<LabeledSpan>,
        file: Arc<String>,
        path: &Path,
    ) -> Result<(), SchemaValidateError> {
        if labels.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidateError::YamlValidate {
                src: NamedSource::new(path.display().to_string(), file).with_language("yaml"),
                labels,
                help: format!(
                    "Try adding these lines to the top of your file for editor validation highlights:\n{}\n{}",
                    "---".bright_green(),
                    format!("# yaml-language-server: $schema={}", self.url).bright_green(),
                ),
            })
        }
    }
}

fn process_basic_output(out: BasicOutput<'_>, spanner: &YamlSpan) -> Vec<LabeledSpan> {
    match out {
        BasicOutput::Valid(_) => Vec::new(),
        BasicOutput::Invalid(errors) => {
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
                            remove_json(&e.error_description().to_string()),
                        )
                    })
                    .collect::<HashSet<_>>();
                let mut e = e.into_iter().collect::<Vec<_>>();
                e.sort_by(|e1, e2| e1.0.as_str().cmp(e2.0.as_str()));
                e
            };

            let mut collection: IndexMap<Location, Vec<String>> = IndexMap::new();

            for (instance_path, err) in errors {
                collection
                    .entry(instance_path)
                    .and_modify(|errs| {
                        errs.push(format!("- {}", err.bold().red()));
                    })
                    .or_insert_with(|| vec![format!("- {}", err.bold().red())]);
            }

            collection
                .into_iter()
                .map(|(key, value)| {
                    LabeledSpan::new_with_span(
                        Some(value.into_iter().collect::<Vec<_>>().join("\n")),
                        spanner.get_span(&key).unwrap(),
                    )
                })
                .collect()
        }
    }
}

fn process_err<'a, I>(errors: I, spanner: &YamlSpan) -> Vec<LabeledSpan>
where
    I: Iterator<Item = ValidationError<'a>>,
{
    errors
        .flat_map(|err| process_anyof_error(&err).unwrap_or_else(|| vec![err]))
        .map(|err| {
            let masked_err = err.masked();
            LabeledSpan::new_primary_with_span(
                Some(masked_err.to_string().bold().red().to_string()),
                spanner
                    .get_span(&Location::from(err.instance_path))
                    .unwrap(),
            )
        })
        .collect()
}

fn process_anyof_error(err: &ValidationError<'_>) -> Option<Vec<ValidationError<'static>>> {
    trace!("to_processed_module_err({err:#?})");
    let ValidationError {
        instance,
        kind,
        instance_path,
        schema_path: _,
    } = err;

    let mut path_iter = instance_path.into_iter();
    let uri = match (kind, path_iter.next_back(), path_iter.next_back()) {
        (
            jsonschema::error::ValidationErrorKind::AnyOf,
            Some(jsonschema::paths::LocationSegment::Index(_)),
            Some(jsonschema::paths::LocationSegment::Property("modules")),
        ) => {
            trace!("FOUND MODULE ANYOF ERROR at {instance_path}");
            if instance.get("source").is_some() {
                Uri::parse(CUSTOM_MODULE_SCHEMA.to_string()).ok()?
            } else if instance.get("from-file").is_some() {
                Uri::parse(IMPORT_MODULE_SCHEMA.to_string()).ok()?
            } else {
                let typ = instance.get("type").and_then(Value::as_str)?;
                let typ = ModuleTypeVersion::from(typ);
                trace!("Module type: {typ}");
                Uri::parse(format!(
                    "{JSON_SCHEMA}/modules/{}-{}.json",
                    typ.typ(),
                    typ.version().unwrap_or("latest")
                ))
                .ok()?
            }
        }
        (
            jsonschema::error::ValidationErrorKind::AnyOf,
            Some(jsonschema::paths::LocationSegment::Index(_)),
            Some(jsonschema::paths::LocationSegment::Property("stages")),
        ) => {
            trace!("FOUND STAGE ANYOF ERROR at {instance_path}");

            if instance.get("from-file").is_some() {
                Uri::parse(IMPORT_MODULE_SCHEMA.to_string()).ok()?
            } else {
                Uri::parse(STAGE_SCHEMA.to_string()).ok()?
            }
        }
        _ => return None,
    };

    trace!("Schema URI: {uri}");
    let schema = ASYNC_RUNTIME.block_on(cache_retrieve(&uri)).ok()?;

    let validator = jsonschema::options()
        .with_retriever(ModuleSchemaRetriever)
        .build(&schema)
        .inspect_err(|e| warn!("{e:#?}"))
        .ok()?;

    Some(
        validator
            .iter_errors(instance)
            .flat_map(|err| process_anyof_error(&err).unwrap_or_else(|| vec![err]))
            .map(|err| {
                let mut err = err.to_owned();
                err.instance_path = instance_path
                    .into_iter()
                    .chain(&err.instance_path)
                    .collect();
                err
            })
            .inspect(|errs| {
                trace!("From error: {err:#?}\nTo error list: {errs:#?}");
            })
            .collect(),
    )
}

fn remove_json<S>(string: &S) -> String
where
    S: ToString,
{
    static REGEX_OBJECT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\{.*\}\s(.*)$").unwrap());
    static REGEX_ARRAY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\[.*\]\s(.*)$").unwrap());

    let string = string.to_string();

    if REGEX_OBJECT.is_match(&string) {
        REGEX_OBJECT.replace_all(string.trim(), "$1").into_owned()
    } else if REGEX_ARRAY.is_match(&string) {
        REGEX_ARRAY.replace_all(string.trim(), "$1").into_owned()
    } else {
        string
    }
}

struct ModuleSchemaRetriever;

impl Retrieve for ModuleSchemaRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ASYNC_RUNTIME.block_on(cache_retrieve(uri))?)
    }
}

#[cached(
    result = true,
    key = "String",
    convert = r#"{ format!("{uri}") }"#,
    sync_writes = "by_key"
)]
async fn cache_retrieve(uri: &Uri<String>) -> miette::Result<Value> {
    let scheme = uri.scheme();
    let path = uri.path();

    #[cfg(not(test))]
    {
        use blue_build_utils::constants::SCHEMA_BASE_URL;

        let uri = match scheme.as_str() {
            "json-schema" => {
                format!("{SCHEMA_BASE_URL}{path}")
            }
            "https" => uri.to_string(),
            scheme => miette::bail!("Unknown scheme {scheme}"),
        };

        log::debug!("Retrieving schema from {}", uri.bold().italic());
        tokio::spawn(blue_build_utils::retry_async(3, 2, async move || {
            let response = reqwest::get(&*uri)
                .await
                .into_diagnostic()
                .with_context(|| format!("Failed to retrieve schema from {uri}"))?;
            let raw_output = response.bytes().await.into_diagnostic()?;
            serde_json::from_slice(&raw_output)
                .into_diagnostic()
                .with_context(|| {
                    format!(
                        "Failed to parse json from {uri}, contents:\n{}",
                        String::from_utf8_lossy(&raw_output)
                    )
                })
                .inspect(|value| trace!("{}:\n{value}", uri.bold().italic()))
        }))
        .await
        .expect("Should join task")
    }

    #[cfg(test)]
    {
        let uri = match scheme.as_str() {
            "json-schema" | "https" => {
                format!("test-files/schema/{path}")
            }
            _ => unreachable!(),
        };

        serde_json::from_slice(
            std::fs::read_to_string(uri)
                .into_diagnostic()
                .context("Failed retrieving sub-schema")?
                .as_bytes(),
        )
        .into_diagnostic()
        .context("Failed deserializing sub-schema")
    }
}

#[cfg(test)]
mod test {
    use blue_build_process_management::ASYNC_RUNTIME;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::recipe(
        "test-files/recipes/recipe-pass.yml",
        "test-files/schema/recipe-v1.json"
    )]
    #[case::stage("test-files/recipes/stage-pass.yml", "test-files/schema/stage-v1.json")]
    #[case::stage_list(
        "test-files/recipes/stage-list-pass.yml",
        "test-files/schema/stage-list-v1.json"
    )]
    #[case::module_list(
        "test-files/recipes/module-list-pass.yml",
        "test-files/schema/module-list-v1.json"
    )]
    #[case::akmods(
        "test-files/recipes/modules/akmods-pass.yml",
        "test-files/schema/modules/akmods-v1.json"
    )]
    #[case::bling(
        "test-files/recipes/modules/bling-pass.yml",
        "test-files/schema/modules/bling-v1.json"
    )]
    #[case::brew(
        "test-files/recipes/modules/brew-pass.yml",
        "test-files/schema/modules/brew-v1.json"
    )]
    #[case::chezmoi(
        "test-files/recipes/modules/chezmoi-pass.yml",
        "test-files/schema/modules/chezmoi-v1.json"
    )]
    #[case::containerfile(
        "test-files/recipes/modules/containerfile-pass.yml",
        "test-files/schema/modules/containerfile-v1.json"
    )]
    #[case::copy(
        "test-files/recipes/modules/copy-pass.yml",
        "test-files/schema/modules/copy-v1.json"
    )]
    #[case::default_flatpaks(
        "test-files/recipes/modules/default-flatpaks-pass.yml",
        "test-files/schema/modules/default-flatpaks-v1.json"
    )]
    #[case::files(
        "test-files/recipes/modules/files-pass.yml",
        "test-files/schema/modules/files-v1.json"
    )]
    #[case::fonts(
        "test-files/recipes/modules/fonts-pass.yml",
        "test-files/schema/modules/fonts-v1.json"
    )]
    #[case::gnome_extensions(
        "test-files/recipes/modules/gnome-extensions-pass.yml",
        "test-files/schema/modules/gnome-extensions-v1.json"
    )]
    #[case::gschema_overrides(
        "test-files/recipes/modules/gschema-overrides-pass.yml",
        "test-files/schema/modules/gschema-overrides-v1.json"
    )]
    #[case::justfiles(
        "test-files/recipes/modules/justfiles-pass.yml",
        "test-files/schema/modules/justfiles-v1.json"
    )]
    #[case::rpm_ostree(
        "test-files/recipes/modules/rpm-ostree-pass.yml",
        "test-files/schema/modules/rpm-ostree-v1.json"
    )]
    #[case::script(
        "test-files/recipes/modules/script-pass.yml",
        "test-files/schema/modules/script-v1.json"
    )]
    #[case::signing(
        "test-files/recipes/modules/signing-pass.yml",
        "test-files/schema/modules/signing-v1.json"
    )]
    #[case::systemd(
        "test-files/recipes/modules/systemd-pass.yml",
        "test-files/schema/modules/systemd-v1.json"
    )]
    #[case::yafti(
        "test-files/recipes/modules/yafti-pass.yml",
        "test-files/schema/modules/yafti-v1.json"
    )]
    fn pass_validation(#[case] file: &str, #[case] schema: &'static str) {
        let validator = ASYNC_RUNTIME
            .block_on(SchemaValidator::builder().url(schema).build())
            .unwrap();

        let file_contents = Arc::new(std::fs::read_to_string(file).unwrap());

        let result = validator.process_validation(file, file_contents);
        dbg!(&result);

        assert!(result.is_ok());
    }

    #[rstest]
    #[case::recipe(
        "test-files/recipes/recipe-fail.yml",
        "test-files/schema/recipe-v1.json",
        6
    )]
    #[case::stage(
        "test-files/recipes/stage-fail.yml",
        "test-files/schema/stage-v1.json",
        2
    )]
    #[case::stage_list(
        "test-files/recipes/stage-list-fail.yml",
        "test-files/schema/stage-list-v1.json",
        2
    )]
    #[case::module_list(
        "test-files/recipes/module-list-fail.yml",
        "test-files/schema/module-list-v1.json",
        35
    )]
    #[case::akmods(
        "test-files/recipes/modules/akmods-fail.yml",
        "test-files/schema/modules/akmods-v1.json",
        1
    )]
    #[case::bling(
        "test-files/recipes/modules/bling-fail.yml",
        "test-files/schema/modules/bling-v1.json",
        1
    )]
    #[case::brew(
        "test-files/recipes/modules/brew-fail.yml",
        "test-files/schema/modules/brew-v1.json",
        3
    )]
    #[case::chezmoi(
        "test-files/recipes/modules/chezmoi-fail.yml",
        "test-files/schema/modules/chezmoi-v1.json",
        3
    )]
    #[case::containerfile(
        "test-files/recipes/modules/containerfile-fail.yml",
        "test-files/schema/modules/containerfile-v1.json",
        2
    )]
    #[case::copy(
        "test-files/recipes/modules/copy-fail.yml",
        "test-files/schema/modules/copy-v1.json",
        2
    )]
    #[case::default_flatpaks(
        "test-files/recipes/modules/default-flatpaks-fail.yml",
        "test-files/schema/modules/default-flatpaks-v1.json",
        4
    )]
    #[case::files(
        "test-files/recipes/modules/files-fail.yml",
        "test-files/schema/modules/files-v1.json",
        1
    )]
    #[case::fonts(
        "test-files/recipes/modules/fonts-fail.yml",
        "test-files/schema/modules/fonts-v1.json",
        2
    )]
    #[case::gnome_extensions(
        "test-files/recipes/modules/gnome-extensions-fail.yml",
        "test-files/schema/modules/gnome-extensions-v1.json",
        2
    )]
    #[case::gschema_overrides(
        "test-files/recipes/modules/gschema-overrides-fail.yml",
        "test-files/schema/modules/gschema-overrides-v1.json",
        1
    )]
    #[case::justfiles(
        "test-files/recipes/modules/justfiles-fail.yml",
        "test-files/schema/modules/justfiles-v1.json",
        2
    )]
    #[case::rpm_ostree(
        "test-files/recipes/modules/rpm-ostree-fail.yml",
        "test-files/schema/modules/rpm-ostree-v1.json",
        3
    )]
    #[case::script(
        "test-files/recipes/modules/script-fail.yml",
        "test-files/schema/modules/script-v1.json",
        2
    )]
    #[case::signing(
        "test-files/recipes/modules/signing-fail.yml",
        "test-files/schema/modules/signing-v1.json",
        1
    )]
    #[case::systemd(
        "test-files/recipes/modules/systemd-fail.yml",
        "test-files/schema/modules/systemd-v1.json",
        4
    )]
    #[case::yafti(
        "test-files/recipes/modules/yafti-fail.yml",
        "test-files/schema/modules/yafti-v1.json",
        1
    )]
    fn fail_validation(#[case] file: &str, #[case] schema: &'static str, #[case] err_count: usize) {
        let validator = ASYNC_RUNTIME
            .block_on(SchemaValidator::builder().url(schema).build())
            .unwrap();

        let file_contents = Arc::new(std::fs::read_to_string(file).unwrap());

        let result = validator.process_validation(file, file_contents);
        dbg!(&result);

        assert!(result.is_err());

        let SchemaValidateError::YamlValidate {
            src: _,
            labels,
            help: _,
        } = result.unwrap_err()
        else {
            panic!("Wrong error");
        };

        assert_eq!(labels.len(), err_count);
    }
}
