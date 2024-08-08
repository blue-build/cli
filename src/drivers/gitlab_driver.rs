use std::env;

use blue_build_utils::{
    constants::{
        CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
        CI_PIPELINE_SOURCE, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_PROJECT_URL, CI_REGISTRY,
        CI_SERVER_HOST, CI_SERVER_PROTOCOL,
    },
    get_env_var,
};
use log::{debug, trace};

use crate::drivers::Driver;

use super::CiDriver;

pub struct GitlabDriver;

impl CiDriver for GitlabDriver {
    fn on_default_branch() -> bool {
        env::var(CI_DEFAULT_BRANCH).is_ok_and(|default_branch| {
            env::var(CI_COMMIT_REF_NAME).is_ok_and(|branch| default_branch == branch)
        })
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        Ok(format!(
            "{}//.gitlab-ci.yml@refs/heads/{}",
            get_env_var(CI_PROJECT_URL)?,
            get_env_var(CI_DEFAULT_BRANCH)?,
        ))
    }

    fn oidc_provider() -> miette::Result<String> {
        Ok(format!(
            "{}://{}",
            get_env_var(CI_SERVER_PROTOCOL)?,
            get_env_var(CI_SERVER_HOST)?,
        ))
    }

    fn generate_tags(recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        let mut tags: Vec<String> = Vec::new();
        let os_version = Driver::get_os_version(recipe)?;

        if Self::on_default_branch() {
            debug!("Running on the default branch");

            tags.push(os_version.to_string());

            let timestamp = blue_build_utils::get_tag_timestamp();
            tags.push(format!("{timestamp}-{os_version}"));

            if let Some(ref alt_tags) = recipe.alt_tags {
                tags.extend(alt_tags.iter().map(ToString::to_string));
            } else {
                tags.push("latest".into());
                tags.push(timestamp);
            }
        } else if let Ok(mr_iid) = env::var(CI_MERGE_REQUEST_IID) {
            trace!("{CI_MERGE_REQUEST_IID}={mr_iid}");

            let pipeline_source = get_env_var(CI_PIPELINE_SOURCE)?;
            trace!("{CI_PIPELINE_SOURCE}={pipeline_source}");

            if pipeline_source == "merge_request_event" {
                debug!("Running in a MR");
                tags.push(format!("mr-{mr_iid}-{os_version}"));
            }
        } else {
            let commit_branch = get_env_var(CI_COMMIT_REF_NAME)?;
            trace!("{CI_COMMIT_REF_NAME}={commit_branch}");

            debug!("Running on branch {commit_branch}");
            tags.push(format!("br-{commit_branch}-{os_version}"));
        }

        let commit_sha = get_env_var(CI_COMMIT_SHORT_SHA)?;
        trace!("{CI_COMMIT_SHORT_SHA}={commit_sha}");

        tags.push(format!("{commit_sha}-{os_version}"));
        Ok(tags)
    }

    fn get_repo_url() -> miette::Result<String> {
        Ok(format!(
            "{}://{}/{}/{}",
            get_env_var(CI_SERVER_PROTOCOL)?,
            get_env_var(CI_SERVER_HOST)?,
            get_env_var(CI_PROJECT_NAMESPACE)?,
            get_env_var(CI_PROJECT_NAME)?,
        ))
    }

    fn get_registry() -> miette::Result<String> {
        Ok(format!(
            "{}/{}/{}",
            get_env_var(CI_REGISTRY)?,
            get_env_var(CI_PROJECT_NAMESPACE)?,
            get_env_var(CI_PROJECT_NAME)?,
        )
        .to_lowercase())
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use blue_build_utils::constants::{
        CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
        CI_PIPELINE_SOURCE, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_REGISTRY, CI_SERVER_HOST,
        CI_SERVER_PROTOCOL,
    };

    use crate::{
        drivers::CiDriver,
        test::{create_test_recipe, BB_UNIT_TEST_MOCK_GET_OS_VERSION, ENV_LOCK},
    };

    use super::GitlabDriver;

    fn setup_default_branch() {
        setup();
        env::set_var(CI_COMMIT_REF_NAME, "main");
    }

    fn setup_mr_branch() {
        setup();
        env::set_var(CI_MERGE_REQUEST_IID, "12");
        env::set_var(CI_PIPELINE_SOURCE, "merge_request_event");
        env::set_var(CI_COMMIT_REF_NAME, "test");
    }

    fn setup_branch() {
        setup();
        env::set_var(CI_COMMIT_REF_NAME, "test");
    }

    fn setup() {
        env::set_var(CI_DEFAULT_BRANCH, "main");
        env::set_var(CI_COMMIT_SHORT_SHA, "1234567");
        env::set_var(CI_REGISTRY, "registry.example.com");
        env::set_var(CI_PROJECT_NAMESPACE, "test-project");
        env::set_var(CI_PROJECT_NAME, "test");
        env::set_var(CI_SERVER_PROTOCOL, "https");
        env::set_var(CI_SERVER_HOST, "gitlab.example.com");
        env::set_var(BB_UNIT_TEST_MOCK_GET_OS_VERSION, "");
    }

    fn teardown() {
        env::remove_var(CI_REGISTRY);
        env::remove_var(CI_PROJECT_NAMESPACE);
        env::remove_var(CI_PROJECT_NAME);
        env::remove_var(CI_DEFAULT_BRANCH);
        env::remove_var(CI_COMMIT_REF_NAME);
        env::remove_var(CI_SERVER_PROTOCOL);
        env::remove_var(CI_SERVER_HOST);
        env::remove_var(BB_UNIT_TEST_MOCK_GET_OS_VERSION);
    }

    #[test]
    fn get_registry() {
        let _env = ENV_LOCK.lock().unwrap();

        setup();

        let registry = GitlabDriver::get_registry().unwrap();

        assert_eq!(registry, "registry.example.com/test-project/test");
        teardown();
    }

    #[test]
    fn on_default_branch_true() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_default_branch();

        assert!(GitlabDriver::on_default_branch());
        teardown();
    }

    #[test]
    fn on_default_branch_false() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_branch();

        assert!(!GitlabDriver::on_default_branch());
        teardown();
    }

    #[test]
    fn get_repo_url() {
        let _env = ENV_LOCK.lock().unwrap();

        setup();

        let url = GitlabDriver::get_repo_url().unwrap();

        assert_eq!(url, "https://gitlab.example.com/test-project/test");
        teardown();
    }

    #[test]
    fn generate_tags_default_branch() {
        let _env = ENV_LOCK.lock().unwrap();
        let timestamp = blue_build_utils::get_tag_timestamp();

        setup_default_branch();

        let mut tags = GitlabDriver::generate_tags(&create_test_recipe()).unwrap();
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

        let mut tags = GitlabDriver::generate_tags(&recipe).unwrap();
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
    fn generate_tags_mr_branch() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_mr_branch();

        let mut tags = GitlabDriver::generate_tags(&create_test_recipe()).unwrap();
        tags.sort();

        let mut expected_tags = vec!["mr-12-40".to_string(), "1234567-40".to_string()];
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }

    #[test]
    fn generate_tags_branch() {
        let _env = ENV_LOCK.lock().unwrap();

        setup_branch();

        let mut tags = GitlabDriver::generate_tags(&create_test_recipe()).unwrap();
        tags.sort();

        let mut expected_tags = vec!["1234567-40".to_string(), "br-test-40".to_string()];
        expected_tags.sort();

        assert_eq!(tags, expected_tags);

        teardown();
    }
}
