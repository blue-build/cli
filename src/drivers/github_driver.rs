use std::{env, fs, path::PathBuf};

use blue_build_utils::constants::GITHUB_EVENT_PATH;
use event::Event;
use log::trace;

use super::CiDriver;

mod event;

pub struct GithubDriver;

impl CiDriver for GithubDriver {
    fn on_main_branch() -> bool {
        env::var(GITHUB_EVENT_PATH)
            .ok()
            .map(PathBuf::from)
            .and_then(|event_path| {
                let event: Event =
                    serde_json::from_str(&fs::read_to_string(event_path).ok()?).ok()?;
                trace!("{event:?}");
                todo!()
            })
            .unwrap_or(false)
    }

    fn cert_identity() -> miette::Result<String> {
        todo!()
    }

    fn generate_tags<T, S>(
        _recipe: &blue_build_recipe::Recipe,
        _alt_tags: Option<T>,
    ) -> miette::Result<Vec<String>>
    where
        T: AsRef<[S]>,
        S: AsRef<str>,
    {
        todo!()
    }

    fn get_repo_url() -> miette::Result<String> {
        todo!()
    }

    fn get_registry() -> miette::Result<String> {
        todo!()
    }
}
