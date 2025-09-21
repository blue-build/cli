use blue_build_utils::{constants::IMAGE_VERSION_LABEL, semver::Version};
use bon::Builder;
use miette::{Context, Result, miette};
use oci_distribution::{config::Config, manifest::OciManifest};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ImageConfig {
    config: Config,
}

#[derive(Debug, Clone, Builder)]
pub struct ImageMetadata {
    manifest: OciManifest,
    digest: String,
    config: ImageConfig,
}

impl ImageMetadata {
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    #[must_use]
    pub const fn manifest(&self) -> &OciManifest {
        &self.manifest
    }

    /// Get the version from the label if possible.
    ///
    /// # Errors
    /// Will error if labels don't exist, the version label
    /// doen't exist, or the version cannot be parsed.
    pub fn get_version(&self) -> Result<Version> {
        self.config
            .config
            .labels
            .as_ref()
            .ok_or_else(|| miette!("No labels found"))?
            .get(IMAGE_VERSION_LABEL)
            .ok_or_else(|| miette!("No version label found"))
            .and_then(|v| {
                v.parse::<Version>()
                    .wrap_err_with(|| format!("Failed to parse version {v}"))
            })
    }
}
