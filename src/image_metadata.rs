use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct ImageMetadata {
    #[serde(alias = "Labels")]
    pub labels: HashMap<String, Value>,

    #[serde(alias = "Digest")]
    pub digest: String,
}

impl ImageMetadata {
    pub fn get_version(&self) -> Option<String> {
        Some(
            self.labels
                .get(IMAGE_VERSION_LABEL)?
                .as_str()
                .map(std::string::ToString::to_string)?
                .split('.')
                .take(1)
                .collect(),
        )
    }
}
