use std::borrow::Cow;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{ModuleExt, StagesExt};

/// Corresponds to a stage in a Containerfile
///
/// A stage has its own list of modules to run which
/// allows the user to reuse the modules thats provided to the main build.
#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Stage<'a> {
    /// The name of the stage.
    ///
    /// This can then be referenced in the `copy`
    /// module using the `from:` property.
    #[builder(setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Cow<'a, str>>,

    /// The base image of the stage.
    ///
    /// This is set directly in a `FROM` instruction.
    #[builder(setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Cow<'a, str>>,

    /// The shell to use in the stage.
    #[builder(setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Vec<Cow<'a, str>>>,

    /// A reference to another recipe file containing
    /// one or more stages.
    ///
    /// An imported recipe file can contain just a single stage like:
    ///
    /// ```yaml
    /// name: blue-build
    /// image: rust
    /// modules:
    /// - type: containerfile
    ///   snippets:
    ///   - |
    ///     RUN cargo install blue-build --debug --all-features --target x86_64-unknown-linux-gnu \
    ///       && mkdir -p /out/ \
    ///       && mv $CARGO_HOME/bin/bluebuild /out/bluebuild
    /// ```
    ///
    /// Or it can contain multiple stages:
    ///
    /// ```yaml
    /// stages:
    /// - name: blue-build
    ///   image: rust
    ///   modules:
    ///   - type: containerfile
    ///     snippets:
    ///     - |
    ///       RUN cargo install blue-build --debug --all-features --target x86_64-unknown-linux-gnu \
    ///         && mkdir -p /out/ \
    ///         && mv $CARGO_HOME/bin/bluebuild /out/bluebuild
    /// - name: hello-world
    ///   image: alpine
    ///   modules:
    ///   - type: script
    ///     snippets:
    ///     - echo "Hello World!"
    /// ```
    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "from-file", skip_serializing_if = "Option::is_none")]
    pub from_file: Option<Cow<'a, str>>,

    /// The modules extension for the stage
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
                        if stage.name.is_some() || stage.image.is_some() || stage.shell.is_some() {
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
