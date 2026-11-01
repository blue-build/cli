use std::iter::once_with;

use blue_build_utils::{constants::IMAGE_VERSION_LABEL, container::Digest, semver::Version};
use bon::Builder;
use miette::{Context, Result, miette};
use oci_client::{config::Config, manifest::OciManifest};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ImageConfig {
    config: Option<Config>,
}

#[derive(Debug, Clone, Builder)]
pub struct ImageMetadata {
    manifest: OciManifest,
    digest: Digest,
    configs: Vec<(String, ImageConfig)>,
}

impl ImageMetadata {
    #[must_use]
    pub const fn digest(&self) -> &Digest {
        &self.digest
    }

    #[must_use]
    pub const fn manifest(&self) -> &OciManifest {
        &self.manifest
    }

    /// Gets all the digests from from the metadata.
    ///
    /// # Errors
    /// Will error if one of the digests are invalid.
    pub fn all_digests(&self) -> Result<Vec<Digest>> {
        let iter = once_with(|| Ok(self.digest.clone()));
        if let OciManifest::ImageIndex(index) = &self.manifest {
            iter.chain(
                index
                    .manifests
                    .iter()
                    .map(|manifest| manifest.digest.parse::<Digest>()),
            )
            .collect()
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
            .find_map(|(_, config)| {
                config
                    .config
                    .as_ref()?
                    .labels
                    .as_ref()?
                    .get(IMAGE_VERSION_LABEL)
            })
            .ok_or_else(|| miette!("No version label found"))
            .and_then(|v| {
                v.parse::<Version>()
                    .wrap_err_with(|| format!("Failed to parse version {v}"))
            })
    }
}
