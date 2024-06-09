use std::{borrow::Cow, path::PathBuf};

use anyhow::{bail, Result};
use blue_build_utils::syntax_highlighting::highlight_ser;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::{Module, ModuleExt, StagesExt};

/// Contains the required fields for a stage.
#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct StageRequiredFields<'a> {
    /// The name of the stage.
    ///
    /// This can then be referenced in the `copy`
    /// module using the `from:` property.
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    /// The base image of the stage.
    ///
    /// This is set directly in a `FROM` instruction.
    #[builder(setter(into))]
    pub from: Cow<'a, str>,

    /// The shell to use in the stage.
    #[builder(default, setter(into, strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Cow<'a, [Cow<'a, str>]>>,

    /// The modules extension for the stage
    #[serde(flatten)]
    pub modules_ext: ModuleExt<'a>,
}

/// Corresponds to a stage in a Containerfile
///
/// A stage has its own list of modules to run which
/// allows the user to reuse the modules thats provided to the main build.
#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Stage<'a> {
    /// The requied fields for a stage.
    #[builder(default, setter(strip_option))]
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub required_fields: Option<StageRequiredFields<'a>>,

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
}

impl<'a> Stage<'a> {
    /// Get's any child stages.
    ///
    /// # Errors
    /// Will error if the stage cannot be
    /// deserialized or the user uses another
    /// property alongside `from-file:`.
    pub fn get_stages(stages: &[Self], traversed_files: Option<Vec<PathBuf>>) -> Result<Vec<Self>> {
        let mut found_stages = vec![];
        let traversed_files = traversed_files.unwrap_or_default();

        for stage in stages {
            found_stages.extend(
                match stage {
                    Stage {
                        required_fields: Some(_),
                        from_file: None,
                    } => vec![stage.clone()],
                    Stage {
                        required_fields: None,
                        from_file: Some(file_name),
                    } => {
                        let file_name = PathBuf::from(file_name.as_ref());
                        if traversed_files.contains(&file_name) {
                            bail!(
                                "{} File {} has already been parsed:\n{traversed_files:?}",
                                "Circular dependency detected!".bright_red(),
                                file_name.display().to_string().bold(),
                            );
                        }
                        let mut tf = traversed_files.clone();
                        tf.push(file_name.clone());

                        Self::get_stages(&StagesExt::parse(&file_name)?.stages, Some(tf))?
                    }
                    _ => {
                        let from_example = Stage::builder().from_file("path/to/stage.yml").build();
                        let stage_example = Self::example();

                        bail!(
                            "Improper format for stage. Must be in the format like:\n{}\n{}\n\n{}",
                            highlight_ser(&stage_example, "yaml", None)?,
                            "or".bold(),
                            highlight_ser(&from_example, "yaml", None)?
                        );
                    }
                }
                .into_iter(),
            );
        }
        Ok(found_stages)
    }

    #[must_use]
    pub fn example() -> Self {
        Stage::builder()
            .required_fields(
                StageRequiredFields::builder()
                    .name("stage-name")
                    .from("build/image:here")
                    .modules_ext(
                        ModuleExt::builder()
                            .modules(vec![Module::example()])
                            .build(),
                    )
                    .build(),
            )
            .build()
    }
}
