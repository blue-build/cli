use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    pub description: String,

    #[serde(alias = "base-image")]
    pub base_image: String,

    #[serde(alias = "image-version")]
    pub image_version: u16,

    pub modules: Vec<Module>,

    pub containerfiles: Option<Containerfiles>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Module {
    #[serde(rename = "type")]
    pub module_type: Option<String>,

    #[serde(rename = "from-file")]
    pub from_file: Option<String>,

    #[serde(flatten)]
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Containerfiles {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}
