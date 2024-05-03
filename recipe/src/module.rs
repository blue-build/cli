use std::{borrow::Cow, process};

use anyhow::{bail, Result};
use indexmap::IndexMap;
use log::{error, trace, warn};
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

    #[builder(default)]
    #[serde(rename = "no-cache", default)]
    pub no_cache: bool,

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

    #[must_use]
    pub fn get_copy_args(&'a self) -> Option<(Option<&'a str>, &'a str, &'a str)> {
        Some((
            self.config.get("from").and_then(|from| from.as_str()),
            self.config.get("src")?.as_str()?,
            self.config.get("dest")?.as_str()?,
        ))
    }

    #[must_use]
    pub fn generate_akmods_info(&'a self, os_version: &u64) -> AkmodsInfo {
        #[derive(Debug, Copy, Clone)]
        enum NvidiaAkmods {
            Nvidia(bool),
            Version(u64),
        }

        trace!("generate_akmods_base({self:#?}, {os_version})");

        let base = self
            .config
            .get("base")
            .map(|b| b.as_str().unwrap_or_default());
        let nvidia = self.config.get("nvidia-version").map_or_else(
            || {
                self.config
                    .get("nvidia")
                    .map_or_else(|| NvidiaAkmods::Nvidia(false), |v| NvidiaAkmods::Nvidia(v.as_bool().unwrap_or_default()))
            },
            |v| {
                warn!(
                    "The `nvidia-version` property is deprecated as upstream images may no longer exist, replace it with `nvidia: true`"
                );
                NvidiaAkmods::Version(v.as_u64().unwrap_or_default())
            },
        );

        AkmodsInfo::builder()
            .images(match (base, nvidia) {
                (Some(b), NvidiaAkmods::Nvidia(nv)) if !b.is_empty() && nv => (
                    format!("akmods:{b}-{os_version}"),
                    format!("akmods-extra:{b}-{os_version}"),
                    Some(format!("akmods-nvidia:{b}-{os_version}")),
                ),
                (Some(b), NvidiaAkmods::Version(nv)) if !b.is_empty() && nv > 0 => (
                    format!("akmods:{b}-{os_version}"),
                    format!("akmods-extra:{b}-{os_version}"),
                    Some(format!("akmods-nvidia:{b}-{os_version}-{nv}")),
                ),
                (Some(b), _) if !b.is_empty() => (
                    format!("akmods:{b}-{os_version}"),
                    format!("akmods-extra:{b}-{os_version}"),
                    None,
                ),
                (_, NvidiaAkmods::Nvidia(nv)) if nv => (
                    format!("akmods:main-{os_version}"),
                    format!("akmods-extra:main-{os_version}"),
                    Some(format!("akmods-nvidia:main-{os_version}")),
                ),
                (_, NvidiaAkmods::Version(nv)) if nv > 0 => (
                    format!("akmods:main-{os_version}"),
                    format!("akmods-extra:main-{os_version}"),
                    Some(format!("akmods-nvidia:main-{os_version}-{nv}")),
                ),
                _ => (
                    format!("akmods:main-{os_version}"),
                    format!("akmods-extra:main-{os_version}"),
                    None,
                ),
            })
            .stage_name(format!(
                "{}{}",
                base.unwrap_or("main"),
                match nvidia {
                    NvidiaAkmods::Nvidia(nv) if nv => "-nvidia".to_string(),
                    NvidiaAkmods::Version(nv) if nv > 0 => format!("-nvidia-{nv}"),
                    _ => String::default(),
                }
            ))
            .build()
    }
}
