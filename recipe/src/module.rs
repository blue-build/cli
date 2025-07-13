use std::{borrow::Cow, path::PathBuf};

use blue_build_utils::{
    constants::BLUE_BUILD_MODULE_IMAGE_REF, secret::Secret, syntax_highlighting::highlight_ser,
};
use bon::Builder;
use colored::Colorize;
use indexmap::IndexMap;
use log::trace;
use miette::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{AkmodsInfo, ModuleExt, base_recipe_path};

mod type_ver;

pub use type_ver::*;

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
pub struct ModuleRequiredFields<'a> {
    #[builder(into)]
    #[serde(rename = "type")]
    pub module_type: ModuleTypeVersion<'a>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Cow<'a, str>>,

    #[builder(default)]
    #[serde(rename = "no-cache", default, skip_serializing_if = "is_false")]
    pub no_cache: bool,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<IndexMap<String, String>>,

    #[builder(into, default)]
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub secrets: Vec<Secret>,

    #[serde(flatten)]
    #[builder(default, into)]
    pub config: IndexMap<String, Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
}

impl<'a> ModuleRequiredFields<'a> {
    #[must_use]
    pub fn get_module_type_list(&'a self, typ: &str, list_key: &str) -> Option<Vec<String>> {
        if self.module_type.typ() == typ {
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
    pub fn get_copy_args(&'a self) -> Option<(Option<&'a str>, &'a str, &'a str)> {
        Some((
            self.config.get("from").and_then(|from| from.as_str()),
            self.config.get("src")?.as_str()?,
            self.config.get("dest")?.as_str()?,
        ))
    }

    #[must_use]
    pub fn get_env(&self) -> Vec<(&String, &String)> {
        self.env
            .as_ref()
            .iter()
            .flat_map(|args| args.iter())
            .collect()
    }

    #[must_use]
    pub fn get_non_local_source(&'a self) -> Option<&'a str> {
        let source = self.source.as_deref()?;

        if source == "local" {
            None
        } else {
            Some(source)
        }
    }

    #[must_use]
    pub fn get_module_image(&self) -> String {
        format!(
            "{BLUE_BUILD_MODULE_IMAGE_REF}/{}:{}",
            self.module_type.typ(),
            self.module_type.version().unwrap_or("latest")
        )
    }

    #[must_use]
    pub fn is_local_source(&self) -> bool {
        self.source
            .as_deref()
            .is_some_and(|source| source == "local")
    }

    #[must_use]
    pub fn generate_akmods_info(&'a self, os_version: &u64) -> AkmodsInfo {
        #[derive(Debug, Default, Copy, Clone)]
        enum NvidiaAkmods {
            #[default]
            Disabled,
            Enabled,
            Open,
            Proprietary,
        }

        impl From<&Value> for NvidiaAkmods {
            fn from(value: &Value) -> Self {
                match value.get("nvidia") {
                    Some(enabled) if enabled.is_bool() => match enabled.as_bool() {
                        Some(true) => Self::Enabled,
                        _ => Self::Disabled,
                    },
                    Some(driver_type) if driver_type.is_string() => match driver_type.as_str() {
                        Some("open") => Self::Open,
                        Some("proprietary") => Self::Proprietary,
                        _ => Self::Disabled,
                    },
                    _ => Self::Disabled,
                }
            }
        }

        trace!("generate_akmods_base({self:#?}, {os_version})");

        let base = self
            .config
            .get("base")
            .map(|b| b.as_str().unwrap_or_default());
        let nvidia = self
            .config
            .get("nvidia")
            .map_or_else(Default::default, NvidiaAkmods::from);

        AkmodsInfo::builder()
            .images(match (base, nvidia) {
                (Some("bazzite"), NvidiaAkmods::Enabled | NvidiaAkmods::Proprietary) => (
                    format!("akmods:bazzite-{os_version}"),
                    Some(format!("akmods-extra:bazzite-{os_version}")),
                    Some(format!("akmods-nvidia:bazzite-{os_version}")),
                ),
                (Some("bazzite"), NvidiaAkmods::Disabled) => (
                    format!("akmods:bazzite-{os_version}"),
                    Some(format!("akmods-extra:bazzite-{os_version}")),
                    None,
                ),
                (Some("bazzite"), NvidiaAkmods::Open) => (
                    format!("akmods:bazzite-{os_version}"),
                    Some(format!("akmods-extra:bazzite-{os_version}")),
                    Some(format!("akmods-nvidia-open:bazzite-{os_version}")),
                ),
                (Some(b), NvidiaAkmods::Enabled | NvidiaAkmods::Proprietary) if !b.is_empty() => (
                    format!("akmods:{b}-{os_version}"),
                    None,
                    Some(format!("akmods-nvidia:{b}-{os_version}")),
                ),
                (Some(b), NvidiaAkmods::Disabled) if !b.is_empty() => {
                    (format!("akmods:{b}-{os_version}"), None, None)
                }
                (Some(b), NvidiaAkmods::Open) if !b.is_empty() => (
                    format!("akmods:{b}-{os_version}"),
                    None,
                    Some(format!("akmods-nvidia-open:{b}-{os_version}")),
                ),
                (_, NvidiaAkmods::Enabled | NvidiaAkmods::Proprietary) => (
                    format!("akmods:main-{os_version}"),
                    None,
                    Some(format!("akmods-nvidia:main-{os_version}")),
                ),
                (_, NvidiaAkmods::Disabled) => (format!("akmods:main-{os_version}"), None, None),
                (_, NvidiaAkmods::Open) => (
                    format!("akmods:main-{os_version}"),
                    None,
                    Some(format!("akmods-nvidia-open:main-{os_version}")),
                ),
            })
            .stage_name(format!(
                "{}{}",
                base.unwrap_or("main"),
                match nvidia {
                    NvidiaAkmods::Disabled => String::default(),
                    _ => "-nvidia".to_string(),
                }
            ))
            .build()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder, Default)]
pub struct Module<'a> {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub required_fields: Option<ModuleRequiredFields<'a>>,

    #[builder(into)]
    #[serde(rename = "from-file", skip_serializing_if = "Option::is_none")]
    pub from_file: Option<Cow<'a, str>>,
}

impl Module<'_> {
    /// Get's any child modules.
    ///
    /// # Errors
    /// Will error if the module cannot be
    /// deserialized or the user uses another
    /// property alongside `from-file:`.
    pub fn get_modules(
        modules: &[Self],
        traversed_files: Option<Vec<PathBuf>>,
    ) -> Result<Vec<Self>> {
        let mut found_modules = vec![];
        let traversed_files = traversed_files.unwrap_or_default();

        for module in modules {
            found_modules.extend(
                match &module {
                    Module {
                        required_fields: Some(_),
                        from_file: None,
                    } => vec![module.clone()],
                    Module {
                        required_fields: None,
                        from_file: Some(file_name),
                    } => {
                        let file_name = PathBuf::from(&**file_name);
                        if traversed_files.contains(&file_name) {
                            bail!(
                                "{} File {} has already been parsed:\n{traversed_files:?}",
                                "Circular dependency detected!".bright_red(),
                                file_name.display().to_string().bold(),
                            );
                        }

                        let mut traversed_files = traversed_files.clone();
                        traversed_files.push(file_name.clone());

                        Self::get_modules(
                            &ModuleExt::try_from(&file_name)?.modules,
                            Some(traversed_files),
                        )?
                    }
                    _ => {
                        let from_example = Self::builder().from_file("test.yml").build();
                        let module_example = Self::example();

                        bail!(
                            "Improper format for module. Must be in the format like:\n{}\n{}\n\n{}",
                            highlight_ser(&module_example, "yaml", None)?,
                            "or".bold(),
                            highlight_ser(&from_example, "yaml", None)?
                        );
                    }
                }
                .into_iter(),
            );
        }
        Ok(found_modules)
    }

    #[must_use]
    pub fn get_from_file_path(&self) -> Option<PathBuf> {
        self.from_file
            .as_ref()
            .map(|path| base_recipe_path().join(&**path))
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn example() -> Self {
        Self::builder()
            .required_fields(
                ModuleRequiredFields::builder()
                    .module_type("script")
                    .config(IndexMap::from_iter([
                        (
                            "snippets".to_string(),
                            Value::Sequence(bon::vec!["echo 'Hello World!'"]),
                        ),
                        (
                            "scripts".to_string(),
                            Value::Sequence(bon::vec!["install-program.sh"]),
                        ),
                    ]))
                    .build(),
            )
            .build()
    }
}
