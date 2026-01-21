use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use blue_build_process_management::ASYNC_RUNTIME;
use bon::bon;
use cached::proc_macro::cached;
use colored::Colorize;
use jsonschema::{Evaluation, Retrieve, Uri, Validator};
use log::trace;
use miette::{Context, IntoDiagnostic, LabeledSpan, NamedSource};
use serde::Deserialize;
use serde_json::Value;

use super::{location::Location, yaml_span::YamlSpan};

mod error;

pub use error::*;

#[derive(Debug, Clone)]
pub struct SchemaValidator {
    validator: Arc<Validator>,
    url: &'static str,
}

#[bon]
impl SchemaValidator {
    #[builder]
    pub async fn new(
        /// The URL of the schema to validate against
        url: &'static str,
    ) -> Result<Self, SchemaValidateBuilderError> {
        tokio::spawn(async move {
            let schema: Value = {
                #[cfg(not(test))]
                {
                    reqwest::get(url)
                        .await
                        .map_err(|e| SchemaValidateBuilderError::Reqwest(url.into(), e))?
                        .json()
                        .await
                        .map_err(|e| SchemaValidateBuilderError::Reqwest(url.into(), e))?
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
                .expect("Should join blocking thread")
                .map_err(|e| SchemaValidateBuilderError::JsonSchemaBuild(url.into(), e))?,
            );

            Ok(Self { validator, url })
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

        process_evaluation(&self.validator.evaluate(&instance), &spanner)
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

fn process_evaluation(
    errors: &Evaluation,
    spanner: &YamlSpan,
) -> Result<Vec<LabeledSpan>, SchemaValidateError> {
    #[derive(Debug, Deserialize)]
    struct EvalList {
        valid: bool,
        details: Vec<EvalEntry>,
    }
    #[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    #[serde(untagged)]
    enum Error {
        Single(String),
        Multi(Vec<String>),
    }
    #[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    #[serde(rename_all = "camelCase")]
    struct EvalEntry {
        valid: bool,
        instance_location: Location,
        errors: Option<BTreeMap<String, Error>>,
    }
    // #[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
    // struct ErrorEntry {
    //     #[serde(rename = "type")]
    //     typ: Option<String>,
    // }

    let errors = serde_json::to_value(errors.list())?;
    // dbg!(&errors);

    let errors: EvalList = serde_json::from_value(errors)?;
    // dbg!(&errors);

    if errors.valid {
        return Ok(Vec::default());
    }

    let errors = errors
        .details
        .into_iter()
        .filter(|entry| !entry.valid && entry.errors.is_some())
        .collect::<BTreeSet<_>>();
    dbg!(&errors);

    Ok(errors
        .into_iter()
        // .filter(|entry| !entry.valid)
        // .collect::<HashSet<EvalEntry>>()
        // .into_iter()
        .filter_map(|entry| {
            Some(LabeledSpan::new_primary_with_span(
                Some(format!("{:?}", entry.errors?)),
                // Some(entry.errors?.typ?),
                spanner.get_span(&entry.instance_location).ok()?,
            ))
        })
        .collect())
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
        let client = reqwest::Client::new();

        log::debug!("Retrieving schema from {}", uri.bold().italic());
        tokio::spawn(blue_build_utils::retry_async(3, 2, async move || {
            let response = client
                .get(&*uri)
                .timeout(std::time::Duration::from_secs(10))
                .send()
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
    use pretty_assertions::assert_eq;
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
