use std::{borrow::Cow, collections::HashMap, ops::Deref};

use blue_build_utils::{
    constants::BLUE_BUILD_DEFAULT_IMAGE, container::Tag, env_str::EnvString, platform::Platform,
};
use bon::Builder;
use miette::{Context, IntoDiagnostic};
use oci_client::Reference;
use serde::{Deserialize, Serialize};
use structstruck::strike;

use crate::{Module, RecipeGetters, RecipeSetters, RecipeV1, Stage};

use super::{MaybeVersion, ModuleExt, StagesExt};

#[derive(Debug, Clone, Serialize)]
pub struct RecipeV2BaseImageStr(EnvString);

impl From<Reference> for RecipeV2BaseImageStr {
    fn from(value: Reference) -> Self {
        Self(EnvString::from(value.to_string()))
    }
}

impl<'de> Deserialize<'de> for RecipeV2BaseImageStr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let image = EnvString::deserialize(deserializer)?;

        if let Err(e) = image.parse::<Reference>() {
            return Err(serde::de::Error::custom(e));
        }

        Ok(Self(image))
    }
}

impl Deref for RecipeV2BaseImageStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

strike! {
    /// The build recipe.
    ///
    /// This is the top-level section of a recipe.yml.
    /// This will contain information on the image and its
    /// base image to assist with building the Containerfile
    /// and tagging the image appropriately.
    #[structstruck::each[derive(Default, Serialize, Clone, Deserialize, Debug, Builder)]]
    #[structstruck::each[builder(on(String, into))]]
    #[structstruck::each[builder(on(EnvString, into))]]
    #[structstruck::each[expect(clippy::duplicated_attributes)]]
    #[structstruck::each[serde(rename_all = "kebab-case")]]
    #[structstruck::each[structstruck::long_names]]
    pub struct RecipeV2 {
        /// Options for the base image like the image and public key.
        pub base: struct {
            /// The base image ref.
            pub image: enum {
                #![derive(Serialize, Clone, Deserialize, Debug)]
                #![structstruck::skip_each]
                #![serde(untagged)]

                /// String representation of an image ref.
                Str(RecipeV2BaseImageStr),

                /// Object representation of an image ref.
                Obj {
                    /// The registry hostname.
                    ///
                    /// i.e. `registry.example.com`
                    registry: EnvString,

                    /// The image repository path in the registry.
                    ///
                    /// i.e. `path/to/image`
                    repository: EnvString,

                    /// The tag of the image. This is overriden by `digest`.
                    /// Defaults to `latest` if left blank.
                    ///
                    /// i.e. `gts`
                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    tag: Option<Tag>,

                    /// The specific digest of the image to pull.
                    /// Overrides the `tag`.
                    ///
                    /// i.e. `sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`
                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    digest: Option<EnvString>,
                },
            },

            /// The public key used to verify the base image signature.
            /// This will validate the image before building with it.
            ///
            /// URLs are supported. Paths are relative to the root of the project.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub public_key: Option<EnvString>,
        },

        /// Metadata for the image like the name, description, and labels.
        pub metadata: struct {
            /// The image name. Used when publishing to GHCR as `ghcr.io/user/name`.
            pub name: EnvString,

            /// The image description. Published to GHCR in the image metadata.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub description: Option<EnvString>,

            /// Allows setting custom tags on the recipe’s final image.
            /// Adding tags to this property will override the `latest` and timestamp tags.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub tags: Vec<Tag>,

            /// A collection of custom labels that will be applied to the image.
            ///
            /// Each item should be a `key: value` pair representing a label name mapping to label value.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub labels: Option<HashMap<String, EnvString>>,
        },

        /// Specifications for the image that modifies how it is built and published.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub spec: Option<pub struct {
           /// Specify a list of the platforms to build for your image.
           /// The resulting images will be added to a manifest list that
           /// allows your host’s container runtime to pull the correct
           /// image architecture for your hardware. The process of
           /// building a multi-architecture image will end up using emulation.
           ///
           /// Consequently, image builds will take significantly longer
           /// and more space will be required on the build host since
           /// each platform that is being built is its own image.
           ///
           /// If `platforms:` is not specified, the build host’s architecture will be used.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub platforms: Vec<Platform>,

            /// Extra tooling version overrides.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub tool_versions: Option<pub struct {
                /// The tag to pull for the BlueBuild cli. This is mostly used for
                /// trying out specific versions of the cli without compiling it locally.
                ///
                /// Supply the tag of the cli release container to pull,
                /// see [the list of available tags](https://github.com/blue-build/cli/pkgs/container/cli) for reference.
                ///
                /// Default: `latest-installer`. Set to `none` to opt out of installing the CLI into your image.
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub bluebuild: Option<MaybeVersion>,

                /// The version of nushell to include at `/usr/libexec/bluebuild/nu/nu` for use by modules in the image.
                ///
                /// This will override the default BlueBuild Nushell version.
                ///
                /// Change only if you need a specific version of Nushell,
                /// changing this might break some BlueBuild modules.
                ///
                /// Nushell is not installed in the image by default.
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub nushell: Option<Tag>,

                /// The version of cosign that will be included in the image.
                ///
                /// This will install the version specified.
                ///
                /// Cosign is not installed in the image by default.
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub cosign: Option<Tag>,
            }>,
        }>,

        /// The stages extension of the recipe.
        ///
        /// This hold the list of stages that can
        /// be used to build software outside of
        /// the final build image.
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub stages_ext: Option<StagesExt>,

        /// The modules extension of the recipe.
        ///
        /// This holds the list of modules to be run on the image.
        #[serde(flatten)]
        pub modules_ext: ModuleExt,
    }
}

impl From<RecipeV1> for RecipeV2 {
    fn from(value: RecipeV1) -> Self {
        Self {
            base: RecipeV2Base {
                image: RecipeV2BaseImage::Str(RecipeV2BaseImageStr(EnvString::from(format!(
                    "{}:{}",
                    value.base_image.unexpanded(),
                    value.image_version.unexpanded(),
                )))),
                public_key: None,
            },
            metadata: RecipeV2Metadata {
                name: value.name,
                description: Some(value.description),
                tags: value.alt_tags.unwrap_or_default(),
                labels: value.labels,
            },
            spec: {
                let has_versions = value.blue_build_tag.is_some()
                    || value.cosign_version.is_some()
                    || value.nushell_version.is_some();
                let tool_versions = has_versions.then_some(RecipeV2SpecToolVersions {
                    bluebuild: value.blue_build_tag,
                    nushell: match value.nushell_version {
                        None | Some(MaybeVersion::None) => None,
                        Some(MaybeVersion::VersionOrBranch(tag)) => Some(tag),
                    },
                    cosign: match value.cosign_version {
                        None | Some(MaybeVersion::None) => None,
                        Some(MaybeVersion::VersionOrBranch(tag)) => Some(tag),
                    },
                });
                match (value.platforms, has_versions) {
                    (None, false) => None,
                    (Some(platforms), false) => Some(RecipeV2Spec {
                        platforms,
                        tool_versions: None,
                    }),
                    (Some(platforms), true) => Some(RecipeV2Spec {
                        platforms,
                        tool_versions,
                    }),
                    (None, true) => Some(RecipeV2Spec {
                        platforms: Vec::new(),
                        tool_versions,
                    }),
                }
            },
            stages_ext: value.stages_ext,
            modules_ext: value.modules_ext,
        }
    }
}

impl Default for RecipeV2BaseImage {
    fn default() -> Self {
        Self::Str(RecipeV2BaseImageStr(BLUE_BUILD_DEFAULT_IMAGE.into()))
    }
}

impl RecipeGetters for RecipeV2 {
    fn get_modules(&self) -> &[Module] {
        &self.modules_ext.modules
    }

    fn get_stages(&self) -> &[Stage] {
        self.stages_ext.as_ref().map_or(&[], |ext| &ext.stages)
    }

    fn get_name(&self) -> &str {
        &self.metadata.name
    }

    fn get_description(&self) -> Option<&str> {
        self.metadata.description.as_deref()
    }

    fn get_labels(&self) -> HashMap<&str, &str> {
        self.metadata
            .labels
            .iter()
            .flatten()
            .map(|(key, value)| (&**key, &**value))
            .collect()
    }

    fn get_alt_tags(&self) -> Option<&[Tag]> {
        match &self.metadata.tags[..] {
            [] => None,
            tags => Some(tags),
        }
    }

    fn get_platforms(&self) -> &[Platform] {
        self.spec.as_ref().map_or(&[], |spec| &spec.platforms)
    }

    fn get_base_image(&self) -> Cow<'_, str> {
        match &self.base.image {
            RecipeV2BaseImage::Str(image) => Cow::Borrowed(
                image
                    .split_once(':') // Split at tag start
                    .or_else(|| image.split_once('@')) // or digest start
                    .unwrap_or((image, "")) // or the image without a tag
                    .0,
            ),
            RecipeV2BaseImage::Obj {
                registry,
                repository,
                ..
            } => Cow::Owned(format!("{registry}/{repository}")),
        }
    }

    fn get_bluebuild_version(&self) -> Option<String> {
        match self
            .spec
            .as_ref()
            .and_then(|spec| spec.tool_versions.as_ref()?.bluebuild.as_ref())
        {
            None => Some("latest-installer".to_string()),
            Some(MaybeVersion::None) => None,
            Some(MaybeVersion::VersionOrBranch(ver)) => Some(format!("{ver}-installer")),
        }
    }

    fn get_cosign_version(&self) -> Option<String> {
        self.spec.as_ref().and_then(|spec| {
            spec.tool_versions
                .as_ref()?
                .cosign
                .as_ref()
                .map(|ver| format!("v{ver}"))
        })
    }

    fn get_nushell_version(&self) -> Option<String> {
        self.spec.as_ref().and_then(|spec| {
            spec.tool_versions
                .as_ref()?
                .nushell
                .as_ref()
                .map(ToString::to_string)
        })
    }

    fn base_image_ref(&self) -> miette::Result<Reference> {
        Ok(match &self.base.image {
            RecipeV2BaseImage::Str(RecipeV2BaseImageStr(image)) => image
                .parse()
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to parse base image ref {image}"))?,
            RecipeV2BaseImage::Obj {
                registry,
                repository,
                tag,
                digest,
            } => digest.as_ref().map_or_else(
                || {
                    Reference::with_tag(
                        registry.to_string(),
                        repository.to_string(),
                        tag.as_ref()
                            .map_or_else(|| "latest".into(), ToString::to_string),
                    )
                },
                |digest| {
                    Reference::with_digest(
                        registry.to_string(),
                        repository.to_string(),
                        digest.to_string(),
                    )
                },
            ),
        })
    }
}

impl RecipeSetters for RecipeV2 {
    fn set_modules(&mut self, modules: Vec<Module>) {
        self.modules_ext.modules = modules;
    }

    fn set_stages(&mut self, stages: Vec<Stage>) {
        if let Some(ext) = self.stages_ext.as_mut() {
            ext.stages = stages;
        } else {
            self.stages_ext = Some(StagesExt::builder().stages(stages).build());
        }
    }
}
