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
                // Handle ublue version formats:
                // - "latest-43.20251123.1" (Aurora/Bluefin)
                // - "43.20251118" (Bazzite)
                // - "43.20251123.1" (Silverblue/Kinoite)

                // Strip any alphabetic prefix and hyphens (e.g., "latest-", "stable-", etc.)
                let version_str = v.trim_start_matches(|c: char| c.is_alphabetic() || c == '-');

                // Extract first component (the Fedora version)
                if let Some(version_part) = version_str.split('.').next() {
                    // Check if it looks like a Fedora version (1-3 digits)
                    if version_part.len() <= 3 && version_part.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(version) = version_part.parse::<Version>() {
                            return Ok(version);
                        }
                    }
                }

                // Fall back to standard semver parsing for non-ublue images
                v.parse::<Version>()
                    .wrap_err_with(|| format!("Failed to deserialize version {v}"))
            })
    }
}
