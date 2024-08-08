use std::{fs, path::PathBuf};

use blue_build_utils::{constants::GITHUB_EVENT_PATH, get_env_var};
use miette::{IntoDiagnostic, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub(super) struct Event {
    pub repository: EventRepository,
    // pub base: Option<EventRefInfo>,
    pub head: Option<EventRefInfo>,

    #[serde(alias = "ref")]
    pub commit_ref: Option<String>,
}

impl Event {
    pub fn try_new() -> Result<Self> {
        get_env_var(GITHUB_EVENT_PATH)
            .map(PathBuf::from)
            .and_then(|event_path| {
                serde_json::from_str::<Self>(&fs::read_to_string(event_path).into_diagnostic()?)
                    .into_diagnostic()
            })
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct EventRepository {
    pub default_branch: String,
    pub owner: EventRepositoryOwner,
    pub html_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct EventRepositoryOwner {
    pub login: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct EventRefInfo {
    #[serde(alias = "ref")]
    pub commit_ref: String,
}
