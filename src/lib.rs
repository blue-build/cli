use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTAINERFILE: &'static str =
    include_str!("../templates/starting_point.template");

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    #[serde(rename = "base-image")]
    pub base_image: String,

    #[serde(rename = "fedora-version")]
    pub fedora_version: u16,

    pub scripts: Scripts,

    pub rpm: Rpm,

    #[serde(rename = "usr-dir-overlays")]
    pub usr_dir_overlays: Option<Vec<String>>,

    pub containerfiles: Option<Containerfiles>,

    pub firstboot: FirstBoot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scripts {
    pub pre: Vec<String>,
    pub post: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rpm {
    pub repos: Vec<String>,
    pub install: Vec<String>,
    pub remove: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirstBoot {
    pub yafti: bool,
    pub flatpaks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Containerfiles {
    pub pre: Vec<String>,
    pub post: Vec<String>,
}
