use std::{borrow::Cow, process};

use anyhow::{bail, Result};
use indexmap::IndexMap;
use log::{error, trace};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::{AkmodsInfo, ModuleExt};

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Module<'a> {
    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub module_type: Option<Cow<'a, str>>,

    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "from-file", skip_serializing_if = "Option::is_none")]
    pub from_file: Option<Cow<'a, str>>,

    #[builder(default, setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Cow<'a, str>>,

    #[serde(flatten)]
    #[builder(default, setter(into))]
    pub config: IndexMap<String, Value>,
}

impl<'a> Module<'a> {
    /// Get's any child modules.
    ///
    /// # Errors
    /// Will error if the module cannot be
    /// deserialized or the user uses another
    /// property alongside `from-file:`.
    pub fn get_modules(modules: &[Self]) -> Result<Vec<Self>> {
        let mut found_modules = vec![];
        for module in modules {
            found_modules.extend(
                match module.from_file.as_ref() {
                    None => vec![module.clone()],
                    Some(file_name) => {
                        if module.module_type.is_some() || module.source.is_some() {
                            bail!("You cannot use the `type:` or `source:` property with `from-file:`");
                        }
                        Self::get_modules(&ModuleExt::parse_module_from_file(file_name)?.modules)?
                    }
                }
                .into_iter(),
            );
        }
        Ok(found_modules)
    }

    #[must_use]
    pub fn get_module_type_list(&'a self, typ: &str, list_key: &str) -> Option<Vec<String>> {
        if self.module_type.as_ref()? == typ {
            Some(
                self.config
                    .get(list_key)?
                    .as_sequence()?
                    .iter()
                    .filter_map(|t| Some(t.as_str()?.to_owned()))
                    .collect(),
            )
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_containerfile_list(&'a self) -> Option<Vec<String>> {
        self.get_module_type_list("containerfile", "containerfiles")
    }

    #[must_use]
    pub fn get_containerfile_snippets(&'a self) -> Option<Vec<String>> {
        self.get_module_type_list("containerfile", "snippets")
    }

    #[must_use]
    pub fn print_module_context(&'a self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            error!("Failed to parse module!!!!!: {e}");
            process::exit(1);
        })
    }

    pub fn generate_akmods_info(&'a self, os_version: &str) -> AkmodsInfo {
        trace!("generate_akmods_base({self:#?}, {os_version})");

        let base = self
            .config
            .get("base")
            .map(|b| b.as_str().unwrap_or_default());
        let nvidia_version = self
            .config
            .get("nvidia-version")
            .map(|v| v.as_u64().unwrap_or_default());

        AkmodsInfo::builder()
            .images(match (base, nvidia_version) {
                (Some(b), Some(nv)) if !b.is_empty() && nv > 0 => (
                    format!("akmods:{b}-{os_version}"),
                    Some(format!("akmods-nvidia:{b}-{os_version}-{nv}")),
                ),
                (Some(b), _) if !b.is_empty() => (format!("akmods:{b}-{os_version}"), None),
                (_, Some(nv)) if nv > 0 => (
                    format!("akmods:main-{os_version}"),
                    Some(format!("akmods-nvidia:main-{os_version}-{nv}")),
                ),
                _ => (format!("akmods:main-{os_version}"), None),
            })
            .stage_name(format!(
                "{}{}",
                base.unwrap_or("main"),
                nvidia_version.map_or_else(String::default, |nv| format!("-{nv}"))
            ))
            .build()
    }
}
