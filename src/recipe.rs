use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    pub name: String,

    #[serde(alias = "base-image")]
    pub base_image: String,

    #[serde(alias = "fedora-version")]
    pub fedora_version: u16,

    pub scripts: Option<Scripts>,

    pub rpm: Option<Rpm>,

    #[serde(alias = "usr-dir-overlays")]
    pub usr_dir_overlays: Option<Vec<String>>,

    pub containerfiles: Option<Containerfiles>,

    pub firstboot: Option<FirstBoot>,
}

impl Recipe {
    pub fn process_repos(mut self) -> Self {
        if let Some(rpm) = &mut self.rpm {
            if let Some(repos) = &rpm.repos {
                rpm.repos = Some(
                    repos
                        .iter()
                        .map(|s| {
                            s.replace("%FEDORA_VERSION%", self.fedora_version.to_string().as_str())
                        })
                        .collect(),
                );
            }
        }
        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scripts {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rpm {
    pub repos: Option<Vec<String>>,
    pub install: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirstBoot {
    pub yafti: bool,
    pub flatpaks: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Containerfiles {
    pub pre: Option<Vec<String>>,
    pub post: Option<Vec<String>>,
}
