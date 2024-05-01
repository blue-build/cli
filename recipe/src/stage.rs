use std::borrow::Cow;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{ModuleExt, StagesExt};

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Stage<'a> {
    #[builder(setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Cow<'a, str>>,

    #[builder(setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Cow<'a, str>>,

    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "from-file", skip_serializing_if = "Option::is_none")]
    pub from_file: Option<Cow<'a, str>>,

    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub modules_ext: Option<ModuleExt<'a>>,
}

impl<'a> Stage<'a> {
    /// Get's any child stages.
    ///
    /// # Errors
    /// Will error if the stage cannot be
    /// deserialized or the user uses another
    /// property alongside `from-file:`.
    pub fn get_stages(stages: &[Self]) -> Result<Vec<Self>> {
        let mut found_stages = vec![];
        for stage in stages {
            found_stages.extend(
                match stage.from_file.as_ref() {
                    None => vec![stage.clone()],
                    Some(file_name) => {
                        if stage.name.is_some() || stage.image.is_some() {
                            bail!(
                                "You cannot use the `name:` or `image:` property with `from-file:`"
                            );
                        }
                        Self::get_stages(&StagesExt::parse_stage_from_file(file_name)?.stages)?
                    }
                }
                .into_iter(),
            );
        }
        Ok(found_stages)
    }
}
