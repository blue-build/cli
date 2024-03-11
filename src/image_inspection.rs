use blue_build_utils::constants::IMAGE_VERSION_LABEL;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct ImageInspection {
    #[serde(alias = "Labels")]
    labels: HashMap<String, Value>,
}

impl ImageInspection {
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
