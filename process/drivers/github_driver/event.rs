use std::{fs, path::PathBuf};

use blue_build_utils::constants::{GITHUB_EVENT_PATH, GITHUB_REF_NAME};
use log::{debug, trace};
use miette::{IntoDiagnostic, Result};
use serde::Deserialize;

#[cfg(test)]
use blue_build_utils::test_utils::get_env_var;

#[cfg(not(test))]
use blue_build_utils::get_env_var;

#[derive(Debug, Deserialize, Clone)]
pub(super) struct Event {
    pub repository: EventRepository,
    // pub base: Option<EventRefInfo>,
    pub head: Option<EventRefInfo>,

    pub pull_request: Option<EventPullRequest>,

    #[serde(alias = "ref")]
    pub commit_ref: Option<String>,
}

impl Event {
    /// Reads from the environment and constructs
    /// itself from the event file.
    ///
    /// # Errors
    /// Will error if the environment variable
    /// `GITHUB_EVENT_PATH` doesn't exist or if
    /// deserializing the event file fails.
    pub fn read_from_env() -> Result<Self> {
        // We cache since the event won't change during evocation
        #[cached::cached()]
        fn inner(path: PathBuf) -> Result<Event> {
            fs::read_to_string(path)
                .into_diagnostic()
                .inspect(|event| trace!("{event}"))
                .and_then(|event| serde_json::from_str(&event).into_diagnostic())
                .inspect(|event| trace!("{event:#?}"))
        }
        get_env_var(GITHUB_EVENT_PATH)
            .map(PathBuf::from)
            .and_then(inner)
    }

    pub fn on_default_branch(&self) -> bool {
        debug!("{self:#?}");

        match (
            self.commit_ref.as_ref(),
            self.head.as_ref(),
            get_env_var(GITHUB_REF_NAME),
        ) {
            (Some(commit_ref), _, _) => {
                commit_ref.trim_start_matches("refs/heads/") == self.repository.default_branch
            }
            (_, Some(head), _) => self.repository.default_branch == head.commit_ref,
            (_, _, Ok(ref_name)) => self.repository.default_branch == ref_name,
            _ => false,
        }
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
pub(super) struct EventPullRequest {
    pub head: EventRefInfo,
}

#[derive(Debug, Deserialize, Clone)]
pub(super) struct EventRefInfo {
    #[serde(alias = "ref")]
    pub commit_ref: String,
    pub repo: EventRepository,
}

#[cfg(test)]
mod test {
    use blue_build_utils::{
        constants::{GITHUB_EVENT_PATH, GITHUB_REF_NAME},
        test_utils::set_env_var,
    };
    use rstest::rstest;

    use super::Event;

    #[rstest]
    #[case::scheduled_main("../test-files/github-events/scheduled.json", "main", true)]
    #[case::push_main("../test-files/github-events/default-branch.json", "main", true)]
    #[case::pr("../test-files/github-events/pr-branch.json", "test", false)]
    #[case::branch("../test-files/github-events/branch.json", "test", false)]
    fn test_on_default_branch(#[case] path: &str, #[case] ref_name: &str, #[case] expected: bool) {
        set_env_var(GITHUB_EVENT_PATH, path);
        set_env_var(GITHUB_REF_NAME, ref_name);

        let event = Event::read_from_env().unwrap();
        eprintln!("{event:?}");

        assert_eq!(expected, event.on_default_branch());
    }
}
