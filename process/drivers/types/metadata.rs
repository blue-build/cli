use std::collections::HashMap;

use blue_build_utils::{constants::IMAGE_VERSION_LABEL, semver::Version};
use log::warn;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ImageMetadata {
    pub labels: HashMap<String, Value>,
    pub digest: String,
}

impl ImageMetadata {
    #[must_use]
    pub fn get_version(&self) -> Option<u64> {
        Some(
            self.labels
                .get(IMAGE_VERSION_LABEL)
                .map(ToOwned::to_owned)
                .and_then(|v| {
                    serde_json::from_value::<Version>(v)
                        .inspect_err(|e| warn!("Failed to parse version:\n{e}"))
                        .ok()
                })?
                .major,
        )
    }
}
