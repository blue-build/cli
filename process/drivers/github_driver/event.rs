use std::path::Path;

use blue_build_utils::constants::GITHUB_REF_NAME;
use log::debug;
use miette::{Context, IntoDiagnostic, Result};
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

    #[serde(alias = "ref")]
    pub commit_ref: Option<String>,
}

impl TryFrom<&str> for Event {
    type Error = miette::Report;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        serde_json::from_str(value).into_diagnostic()
    }
}

impl Event {
    pub fn try_new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        fn inner(path: &Path) -> Result<Event> {
            let contents = std::fs::read_to_string(path)
                .into_diagnostic()
                .with_context(|| format!("Path: {}", path.display()))?;
            Event::try_from(contents.as_str())
        }
        inner(path.as_ref())
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

// impl Event {
//     pub fn try_new() -> Result<Self> {
//         get_env_var(GITHUB_EVENT_PATH)
//             .map(PathBuf::from)
//             .and_then(|event_path| {
//                 serde_json::from_str::<Self>(&fs::read_to_string(event_path).into_diagnostic()?)
//                     .into_diagnostic()
//             })
//     }
// }

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

#[cfg(test)]
mod test {
    use blue_build_utils::{constants::GITHUB_REF_NAME, test_utils::set_env_var};
    use rstest::rstest;

    use super::Event;

    #[rstest]
    #[case::scheduled_main("../test-files/github-events/scheduled.json", "main", true)]
    #[case::push_main("../test-files/github-events/default-branch.json", "main", true)]
    #[case::pr("../test-files/github-events/pr-branch.json", "test", false)]
    #[case::branch("../test-files/github-events/branch.json", "test", false)]
    fn test_on_default_branch(#[case] path: &str, #[case] ref_name: &str, #[case] expected: bool) {
        set_env_var(GITHUB_REF_NAME, ref_name);

        let event = Event::try_new(path).unwrap();
        eprintln!("{event:?}");

        assert_eq!(expected, event.on_default_branch());
    }
}
