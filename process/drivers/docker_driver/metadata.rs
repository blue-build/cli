use std::collections::HashMap;

use blue_build_utils::platform::Platform;
use miette::{Report, bail};
use serde::Deserialize;

use crate::drivers::types::ImageMetadata;

#[derive(Deserialize, Debug, Clone)]
pub struct Metadata {
    manifest: Manifest,
    image: MetadataImage,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlatformManifest {
    digest: String,
    platform: PlatformManifestInfo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlatformManifestInfo {
    architecture: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Manifest {
    digest: String,

    #[serde(default)]
    manifests: Vec<PlatformManifest>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MetadataPlatformImage {
    config: Config,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MetadataImage {
    Single(MetadataPlatformImage),
    Multi(HashMap<String, MetadataPlatformImage>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Config {
    labels: HashMap<String, serde_json::Value>,
}

impl TryFrom<(Metadata, Option<Platform>)> for ImageMetadata {
    type Error = Report;

    fn try_from((metadata, platform): (Metadata, Option<Platform>)) -> Result<Self, Self::Error> {
        match metadata.image {
            MetadataImage::Single(image) => Ok(Self {
                labels: image.config.labels,
                digest: metadata.manifest.digest,
            }),
            MetadataImage::Multi(mut platforms) => {
                let platform = platform.unwrap_or_default();
                let Some(image) = platforms.remove(&platform.to_string()) else {
                    bail!("Image information does not exist for {platform}");
                };
                let Some(manifest) = metadata
                    .manifest
                    .manifests
                    .into_iter()
                    .find(|manifest| manifest.platform.architecture == platform.arch())
                else {
                    bail!("Manifest does not exist for {platform}");
                };
                Ok(Self {
                    labels: image.config.labels,
                    digest: manifest.digest,
                })
            }
        }
    }
}
