use std::sync::Arc;

use blue_build_process_management::ASYNC_RUNTIME;
use cached::proc_macro::cached;
use colored::Colorize;
use jsonschema::{Retrieve, Uri, Validator};
use log::{debug, trace};
use miette::{bail, Context, IntoDiagnostic, Report};
use serde_json::Value;

pub const BASE_SCHEMA_URL: &str = "https://schema.blue-build.org";
pub const RECIPE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/recipe-v1.json";
pub const STAGE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/stage-v1.json";
pub const STAGE_LIST_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/stage-list-v1.json";
pub const MODULE_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/module-v1.json";
pub const MODULE_LIST_V1_SCHEMA_URL: &str = "https://schema.blue-build.org/module-list-v1.json";

#[derive(Debug, Clone)]
pub struct SchemaValidator {
    schema: Arc<Value>,
    validator: Arc<Validator>,
}

impl SchemaValidator {
    pub fn validator(&self) -> Arc<Validator> {
        self.validator.clone()
    }

    pub fn schema(&self) -> Arc<Value> {
        self.schema.clone()
    }
}

pub async fn build_validator(url: &'static str) -> Result<SchemaValidator, Report> {
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

        Ok(SchemaValidator { schema, validator })
    })
    .await
    .expect("Should join task")
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
