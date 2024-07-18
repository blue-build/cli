use blue_build_utils::{
    constants::{
        GITHUB_EVENT_NAME, GITHUB_REF_NAME, GITHUB_SHA, GITHUB_TOKEN_ISSUER_URL,
        GITHUB_WORKFLOW_REF, PR_EVENT_NUMBER,
    },
    get_env_var,
};
use event::Event;
use log::trace;

use super::{CiDriver, Driver};

mod event;

pub struct GithubDriver;

impl CiDriver for GithubDriver {
    fn on_default_branch() -> bool {
        Event::try_new().map_or_else(
            |_| false,
            |event| match (event.commit_ref, event.head) {
                (Some(commit_ref), _) => {
                    commit_ref.trim_start_matches("refs/heads/") == event.repository.default_branch
                }
                (_, Some(head)) => event.repository.default_branch == head.commit_ref,
                _ => false,
            },
        )
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        get_env_var(GITHUB_WORKFLOW_REF)
    }

    fn oidc_provider() -> miette::Result<String> {
        Ok(GITHUB_TOKEN_ISSUER_URL.to_string())
    }

    fn generate_tags(recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        let mut tags: Vec<String> = Vec::new();
        let os_version = Driver::get_os_version(recipe)?;
        let github_event_name = get_env_var(GITHUB_EVENT_NAME)?;

        if github_event_name == "pull_request" {
            trace!("Running in a PR");

            let github_event_number = get_env_var(PR_EVENT_NUMBER)?;

            tags.push(format!("pr-{github_event_number}-{os_version}"));
        } else if Self::on_default_branch() {
            tags.push(os_version.to_string());

            let timestamp = blue_build_utils::get_tag_timestamp();
            tags.push(format!("{timestamp}-{os_version}"));

            if let Some(ref alt_tags) = recipe.alt_tags {
                tags.extend(alt_tags.iter().map(ToString::to_string));
            } else {
                tags.push("latest".into());
                tags.push(timestamp);
            }
        } else {
            let github_ref_name = get_env_var(GITHUB_REF_NAME)?;

            tags.push(format!("br-{github_ref_name}-{os_version}"));
        }

        let mut short_sha = get_env_var(GITHUB_SHA)?;
        short_sha.truncate(7);

        tags.push(format!("{short_sha}-{os_version}"));

        Ok(tags)
    }

    fn get_repo_url() -> miette::Result<String> {
        Ok(Event::try_new()?.repository.html_url)
    }

    fn get_registry() -> miette::Result<String> {
        Ok(format!(
            "ghcr.io/{}",
            Event::try_new()?.repository.owner.login
        ))
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use blue_build_utils::constants::{
        GITHUB_EVENT_NAME, GITHUB_EVENT_PATH, GITHUB_REF_NAME, GITHUB_SHA, PR_EVENT_NUMBER,
    };

    use crate::{
        drivers::CiDriver,
        test::{create_test_recipe, BB_UNIT_TEST_MOCK_GET_OS_VERSION, ENV_LOCK},
    };

    use super::GithubDriver;

    fn setup_default_branch() {
        setup();
        env::set_var(
            GITHUB_EVENT_PATH,
            "../test-files/github-events/default-branch.json",
        );
        env::set_var(GITHUB_REF_NAME, "main");
    }

    fn setup_pr_branch() {
        setup();
        env::set_var(
            GITHUB_EVENT_PATH,
            "../test-files/github-events/pr-branch.json",
        );
        env::set_var(GITHUB_EVENT_NAME, "pull_request");
        env::set_var(GITHUB_REF_NAME, "test");
        env::set_var(PR_EVENT_NUMBER, "12");
    }

    fn setup_branch() {
        setup();
        env::set_var(GITHUB_EVENT_PATH, "../test-files/github-events/branch.json");
        env::set_var(GITHUB_REF_NAME, "test");
    }

    fn setup() {
        env::set_var(GITHUB_EVENT_NAME, "push");
        env::set_var(GITHUB_SHA, "1234567890");
        env::set_var(BB_UNIT_TEST_MOCK_GET_OS_VERSION, "");
    }

    fn teardown() {
        env::remove_var(GITHUB_EVENT_NAME);
        env::remove_var(GITHUB_EVENT_PATH);
        env::remove_var(GITHUB_REF_NAME);
        env::remove_var(GITHUB_SHA);
        env::remove_var(BB_UNIT_TEST_MOCK_GET_OS_VERSION);
    }

    #[test]
    fn get_registry() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_default_branch();

        let registry = GithubDriver::get_registry().unwrap();

        assert_eq!(registry, "ghcr.io/test-owner");
        teardown();
    }

    #[test]
    fn on_default_branch_true() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_default_branch();

        assert!(GithubDriver::on_default_branch());
        teardown();
    }

    #[test]
    fn on_default_branch_false() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_pr_branch();

        assert!(!GithubDriver::on_default_branch());
        teardown();
    }

    #[test]
    fn get_repo_url() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_branch();

        let url = GithubDriver::get_repo_url().unwrap();

        assert_eq!(url, "https://example.com/");
        teardown();
    }

    #[test]
    fn generate_tags_default_branch() {
        let _env = ENV_LOCK.lock().unwrap();
        let timestamp = blue_build_utils::get_tag_timestamp();

        setup_default_branch();

        let mut tags = GithubDriver::generate_tags(&create_test_recipe()).unwrap();
        tags.sort();

        let mut expected_tags = vec![
            format!("{timestamp}-40"),
            "latest".to_string(),
            timestamp,
            "1234567-40".to_string(),
            "40".to_string(),
        ];
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }

    #[test]
    fn generate_tags_default_branch_alt_tags() {
        let _env = ENV_LOCK.lock().unwrap();
        let timestamp = blue_build_utils::get_tag_timestamp();

        setup_default_branch();

        let mut recipe = create_test_recipe();

        recipe.alt_tags = Some(vec!["test-tag1".into(), "test-tag2".into()]);

        let mut tags = GithubDriver::generate_tags(&recipe).unwrap();
        tags.sort();

        let mut expected_tags = vec![
            format!("{timestamp}-40"),
            "1234567-40".to_string(),
            "40".to_string(),
        ];
        expected_tags.extend(recipe.alt_tags.unwrap().iter().map(ToString::to_string));
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }

    #[test]
    fn generate_tags_pr_branch() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_pr_branch();

        let mut tags = GithubDriver::generate_tags(&create_test_recipe()).unwrap();
        tags.sort();

        let mut expected_tags = vec!["pr-12-40".to_string(), "1234567-40".to_string()];
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }

    #[test]
    fn generate_tags_branch() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_branch();

        let mut tags = GithubDriver::generate_tags(&create_test_recipe()).unwrap();
        tags.sort();

        let mut expected_tags = vec!["1234567-40".to_string(), "br-test-40".to_string()];
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }
}
