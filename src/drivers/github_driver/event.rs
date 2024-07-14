use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct Event {
    pub repository: EventRepository,
    // pub base: Option<EventRefInfo>,
    pub head: Option<EventRefInfo>,

    #[serde(alias = "ref")]
    pub commit_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct EventRepository {
    pub default_branch: String,
    // pub owner: EventRepositoryOwner,
}

// #[derive(Debug, Deserialize)]
// pub(super) struct EventRepositoryOwner {
//     pub login: String,
// }

#[derive(Debug, Deserialize)]
pub(super) struct EventRefInfo {
    #[serde(alias = "ref")]
    pub commit_ref: String,
}
