use std::{borrow::Cow, process};

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
    #[must_use]
    pub fn get_modules(modules: &[Self]) -> Vec<Self> {
        modules
            .iter()
            .flat_map(|module| {
                module.from_file.as_ref().map_or_else(
                    || vec![module.clone()],
                    |file_name| match ModuleExt::parse_module_from_file(file_name) {
                        Err(e) => {
                            error!("Failed to get module from {file_name}: {e}");
                            vec![]
                        }
                        Ok(module_ext) => Self::get_modules(&module_ext.modules),
                    },
                )
            })
            .collect()
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

    pub fn print_module_context(&'a self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            error!("Failed to parse module!!!!!: {e}");
            process::exit(1);
        })
    }

    pub fn get_files_list(&'a self) -> Option<Vec<(String, String)>> {
        Some(
            self.config
                .get("files")?
                .as_sequence()?
                .iter()
                .filter_map(|entry| entry.as_mapping())
                .flatten()
                .filter_map(|(src, dest)| {
                    Some((
                        format!("./config/files/{}", src.as_str()?),
                        dest.as_str()?.to_string(),
                    ))
                })
                .collect(),
        )
    }

    pub fn generate_akmods_info(&'a self, os_version: &str) -> AkmodsInfo {
        trace!("generate_akmods_base({self:#?}, {os_version})");

        // `get_os_version` will default to `image_version` which is "latest" in some cases
        let os_version = if os_version == "latest" {
            "39"
        } else {
            os_version
        };

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
