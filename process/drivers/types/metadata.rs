use std::iter::once;

use blue_build_utils::{constants::IMAGE_VERSION_LABEL, semver::Version};
use bon::Builder;
use miette::{Context, Result, miette};
use oci_client::{config::Config, manifest::OciManifest};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ImageConfig {
    config: Config,
}

#[derive(Debug, Clone, Builder)]
pub struct ImageMetadata {
    manifest: OciManifest,
    digest: String,
    configs: Vec<(String, ImageConfig)>,
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

    #[must_use]
    pub fn all_digests(&self) -> Vec<String> {
        let iter = once(self.digest.clone());
        if let OciManifest::ImageIndex(index) = &self.manifest {
            iter.chain(
                index
                    .manifests
                    .iter()
                    .flat_map(|manifest| vec![manifest.digest.clone()]),
            )
            .collect::<Vec<_>>()
        } else {
            iter.collect()
        }
    }

    /// Get the version from the label if possible.
    ///
    /// # Errors
    /// Will error if labels don't exist, the version label
    /// doen't exist, or the version cannot be parsed.
    pub fn get_version(&self) -> Result<Version> {
        self.configs
            .iter()
            .find_map(|(_, config)| config.config.labels.as_ref()?.get(IMAGE_VERSION_LABEL))
            .ok_or_else(|| miette!("No version label found"))
            .and_then(|v| {
                v.parse::<Version>()
                    .wrap_err_with(|| format!("Failed to parse version {v}"))
            })
    }
}
