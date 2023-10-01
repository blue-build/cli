use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    #[serde(alias = "base-image")]
    pub base_image: String,

    #[serde(alias = "fedora-version")]
    pub fedora_version: u16,

    pub scripts: Scripts,

    pub rpm: Rpm,

    #[serde(alias = "usr-dir-overlays")]
    pub usr_dir_overlays: Option<Vec<String>>,

    pub containerfiles: Option<Containerfiles>,

    pub firstboot: FirstBoot,
}

impl Recipe {
    pub fn process_repos(mut self) -> Self {
        self.rpm.repos = self
            .rpm
            .repos
            .iter()
            .map(|s| s.replace("%FEDORA_VERSION%", self.fedora_version.to_string().as_str()))
            .collect();
        self
    }
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
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}
