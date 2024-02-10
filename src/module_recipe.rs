use std::{
    borrow::Cow,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
use chrono::Local;
use format_serde_error::SerdeError;
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::ops::{self, check_command_exists};

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct Recipe<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    #[builder(setter(into))]
    pub description: Cow<'a, str>,

    #[serde(alias = "base-image")]
    #[builder(setter(into))]
    pub base_image: Cow<'a, str>,

    #[serde(alias = "image-version")]
    #[builder(setter(into))]
    pub image_version: Cow<'a, str>,

    #[serde(alias = "blue-build-tag")]
    #[builder(default, setter(into, strip_option))]
    pub blue_build_tag: Option<Cow<'a, str>>,

    #[serde(flatten)]
    pub modules_ext: ModuleExt,

    #[serde(flatten)]
    #[builder(setter(into))]
    pub extra: IndexMap<String, Value>,
}

impl<'a> Recipe<'a> {
    #[must_use]
    pub fn generate_tags(&self) -> Vec<String> {
        trace!("Recipe::generate_tags()");
        trace!("Generating image tags for {}", &self.name);

        let mut tags: Vec<String> = Vec::new();
        let image_version = self.get_os_version();
        let timestamp = Local::now().format("%Y%m%d").to_string();

        if let (Ok(commit_branch), Ok(default_branch), Ok(commit_sha), Ok(pipeline_source)) = (
            env::var("CI_COMMIT_REF_NAME"),
            env::var("CI_DEFAULT_BRANCH"),
            env::var("CI_COMMIT_SHORT_SHA"),
            env::var("CI_PIPELINE_SOURCE"),
        ) {
            trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_DEFAULT_BRANCH={default_branch},CI_COMMIT_SHORT_SHA={commit_sha}, CI_PIPELINE_SOURCE={pipeline_source}");
            warn!("Detected running in Gitlab, pulling information from CI variables");

            if let Ok(mr_iid) = env::var("CI_MERGE_REQUEST_IID") {
                trace!("CI_MERGE_REQUEST_IID={mr_iid}");
                if pipeline_source == "merge_request_event" {
                    debug!("Running in a MR");
                    tags.push(format!("mr-{mr_iid}-{image_version}"));
                }
            }

            if default_branch == commit_branch {
                debug!("Running on the default branch");
                tags.push(image_version.to_string());
                tags.push(format!("{timestamp}-{image_version}"));
                tags.push("latest".into());
                tags.push(timestamp);
            } else {
                debug!("Running on branch {commit_branch}");
                tags.push(format!("br-{commit_branch}-{image_version}"));
            }

            tags.push(format!("{commit_sha}-{image_version}"));
        } else if let (
            Ok(github_event_name),
            Ok(github_event_number),
            Ok(github_sha),
            Ok(github_ref_name),
        ) = (
            env::var("GITHUB_EVENT_NAME"),
            env::var("PR_EVENT_NUMBER"),
            env::var("GITHUB_SHA"),
            env::var("GITHUB_REF_NAME"),
        ) {
            trace!("GITHUB_EVENT_NAME={github_event_name},PR_EVENT_NUMBER={github_event_number},GITHUB_SHA={github_sha},GITHUB_REF_NAME={github_ref_name}");
            warn!("Detected running in Github, pulling information from GITHUB variables");

            let mut short_sha = github_sha;
            short_sha.truncate(7);

            if github_event_name == "pull_request" {
                debug!("Running in a PR");
                tags.push(format!("pr-{github_event_number}-{image_version}"));
            } else if github_ref_name == "live" || github_ref_name == "main" {
                tags.push(image_version.to_string());
                tags.push(format!("{timestamp}-{image_version}"));
                tags.push("latest".into());
                tags.push(timestamp);
            } else {
                tags.push(format!("br-{github_ref_name}-{image_version}"));
            }
            tags.push(format!("{short_sha}-{image_version}"));
        } else {
            warn!("Running locally");
            tags.push(format!("local-{image_version}"));
        }
        debug!("Finished generating tags!");
        debug!("Tags: {tags:#?}");

        tags
    }

    /// # Parse a recipe file
    /// #
    /// # Errors
    pub fn parse<P: AsRef<Path>>(path: &P) -> Result<Self> {
        let file_path = if Path::new(path.as_ref()).is_absolute() {
            path.as_ref().to_path_buf()
        } else {
            std::env::current_dir()?.join(path.as_ref())
        };

        let recipe_path = fs::canonicalize(file_path)?;
        let recipe_path_string = recipe_path.display().to_string();
        debug!("Recipe::parse_recipe({recipe_path_string})");

        let file = fs::read_to_string(recipe_path)?;

        debug!("Recipe contents: {file}");

        let mut recipe =
            serde_yaml::from_str::<Recipe>(&file).map_err(ops::serde_yaml_err(&file))?;

        recipe.modules_ext.modules = Module::get_modules(&recipe.modules_ext.modules);

        Ok(recipe)
    }

    fn get_os_version(&self) -> String {
        trace!("Recipe::get_os_version()");

        if check_command_exists("skopeo").is_err() {
            warn!("The 'skopeo' command doesn't exist, falling back to version defined in recipe");
            return self.image_version.to_string();
        }

        let base_image = self.base_image.as_ref();
        let image_version = self.image_version.as_ref();

        info!("Retrieving information from {base_image}:{image_version}, this will take a bit");

        let output = match Command::new("skopeo")
            .arg("inspect")
            .arg(format!("docker://{base_image}:{image_version}"))
            .output()
        {
            Err(_) => {
                warn!(
                    "Issue running the 'skopeo' command, falling back to version defined in recipe"
                );
                return self.image_version.to_string();
            }
            Ok(output) => output,
        };

        if !output.status.success() {
            warn!("Failed to get image information for {base_image}:{image_version}, falling back to version defined in recipe");
            return self.image_version.to_string();
        }

        let inspection: ImageInspection = match serde_json::from_str(
            String::from_utf8_lossy(&output.stdout).as_ref(),
        ) {
            Err(err) => {
                let err_msg =
                    SerdeError::new(String::from_utf8_lossy(&output.stdout).to_string(), err)
                        .to_string();
                warn!("Issue deserializing 'skopeo' output, falling back to version defined in recipe. {err_msg}",);
                return self.image_version.to_string();
            }
            Ok(inspection) => inspection,
        };

        inspection.get_version().unwrap_or_else(|| {
            warn!("Version label does not exist on image, using version in recipe");
            image_version.to_string()
        })
    }
}

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct ModuleExt {
    #[builder(default, setter(into))]
    pub modules: Vec<Module>,
}

impl ModuleExt {
    /// # Parse a module file returning a [`ModuleExt`]
    ///
    /// # Errors
    /// Can return an `anyhow` Error if the file cannot be read or deserialized
    /// into a [`ModuleExt`]
    pub fn parse_module_from_file(file_name: &str) -> Result<Self> {
        let file_path = PathBuf::from("config").join(file_name);
        let file_path = if file_path.is_absolute() {
            file_path
        } else {
            std::env::current_dir()?.join(file_path)
        };

        let file = fs::read_to_string(file_path)?;

        serde_yaml::from_str::<Self>(&file).map_or_else(
            |_| -> Result<Self> {
                let module =
                    serde_yaml::from_str::<Module>(&file).map_err(ops::serde_yaml_err(&file))?;
                Ok(Self::builder().modules(vec![module]).build())
            },
            Ok,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Module {
    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub module_type: Option<String>,

    #[builder(default, setter(into, strip_option))]
    #[serde(rename = "from-file", skip_serializing_if = "Option::is_none")]
    pub from_file: Option<String>,

    #[serde(flatten)]
    #[builder(default, setter(into))]
    pub config: IndexMap<String, Value>,
}

impl Module {
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
}

#[derive(Deserialize, Debug, Clone)]
struct ImageInspection {
    #[serde(alias = "Labels")]
    labels: HashMap<String, JsonValue>,
}

impl ImageInspection {
    pub fn get_version(&self) -> Option<String> {
        Some(
            self.labels
                .get("org.opencontainers.image.version")?
                .as_str()
                .map(std::string::ToString::to_string)?
                .split('.')
                .take(1)
                .collect(),
        )
    }
}
