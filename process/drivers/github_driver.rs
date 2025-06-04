use std::path::PathBuf;

use blue_build_utils::{
    constants::{
        GITHUB_EVENT_NAME, GITHUB_EVENT_PATH, GITHUB_REF_NAME, GITHUB_SHA, GITHUB_TOKEN_ISSUER_URL,
        GITHUB_WORKFLOW_REF, PR_EVENT_NUMBER,
    },
    string_vec,
};
use event::Event;
use log::trace;

#[cfg(not(test))]
use blue_build_utils::get_env_var;

#[cfg(test)]
use blue_build_utils::test_utils::get_env_var;

use super::{CiDriver, Driver, opts::GenerateTagsOpts};

mod event;

pub struct GithubDriver;

impl CiDriver for GithubDriver {
    fn on_default_branch() -> bool {
        get_env_var(GITHUB_EVENT_PATH)
            .is_ok_and(|path| Event::try_new(path).is_ok_and(|e| e.on_default_branch()))
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        get_env_var(GITHUB_WORKFLOW_REF)
    }

    fn oidc_provider() -> miette::Result<String> {
        Ok(GITHUB_TOKEN_ISSUER_URL.to_string())
    }

    fn generate_tags(opts: GenerateTagsOpts) -> miette::Result<Vec<String>> {
        const PR_EVENT: &str = "pull_request";
        let timestamp = blue_build_utils::get_tag_timestamp();
        let os_version = Driver::get_os_version()
            .oci_ref(opts.oci_ref)
            .platform(opts.platform)
            .call()
            .inspect(|v| trace!("os_version={v}"))?;
        let ref_name = get_env_var(GITHUB_REF_NAME)
            .inspect(|v| trace!("{GITHUB_REF_NAME}={v}"))?
            .replace('/', "_");
        let short_sha = {
            let mut short_sha = get_env_var(GITHUB_SHA).inspect(|v| trace!("{GITHUB_SHA}={v}"))?;
            short_sha.truncate(7);
            short_sha
        };

        let tags = match (
            Self::on_default_branch(),
            opts.alt_tags.as_ref(),
            get_env_var(GITHUB_EVENT_NAME).inspect(|v| trace!("{GITHUB_EVENT_NAME}={v}")),
            get_env_var(PR_EVENT_NUMBER).inspect(|v| trace!("{PR_EVENT_NUMBER}={v}")),
        ) {
            (true, None, _, _) => {
                string_vec![
                    "latest",
                    &timestamp,
                    format!("{os_version}"),
                    format!("{timestamp}-{os_version}"),
                    format!("{short_sha}-{os_version}"),
                ]
            }
            (true, Some(alt_tags), _, _) => alt_tags
                .iter()
                .flat_map(|alt| {
                    string_vec![
                        &**alt,
                        format!("{alt}-{os_version}"),
                        format!("{timestamp}-{alt}-{os_version}"),
                        format!("{short_sha}-{alt}-{os_version}"),
                    ]
                })
                .collect(),
            (false, None, Ok(event_name), Ok(event_num)) if event_name == PR_EVENT => {
                vec![
                    format!("pr-{event_num}-{os_version}"),
                    format!("{short_sha}-{os_version}"),
                ]
            }
            (false, None, _, _) => {
                vec![
                    format!("br-{ref_name}-{os_version}"),
                    format!("{short_sha}-{os_version}"),
                ]
            }
            (false, Some(alt_tags), Ok(event_name), Ok(event_num)) if event_name == PR_EVENT => {
                alt_tags
                    .iter()
                    .flat_map(|alt| {
                        vec![
                            format!("pr-{event_num}-{alt}-{os_version}"),
                            format!("{short_sha}-{alt}-{os_version}"),
                        ]
                    })
                    .collect()
            }
            (false, Some(alt_tags), _, _) => alt_tags
                .iter()
                .flat_map(|alt| {
                    vec![
                        format!("br-{ref_name}-{alt}-{os_version}"),
                        format!("{short_sha}-{alt}-{os_version}"),
                    ]
                })
                .collect(),
        };
        trace!("{tags:?}");

        Ok(tags)
    }

    fn get_repo_url() -> miette::Result<String> {
        Ok(Event::try_new(get_env_var(GITHUB_EVENT_PATH)?)?
            .repository
            .html_url)
    }

    fn get_registry() -> miette::Result<String> {
        Ok(format!(
            "ghcr.io/{}",
            Event::try_new(get_env_var(GITHUB_EVENT_PATH)?)?
                .repository
                .owner
                .login
        )
        .trim()
        .to_lowercase())
    }

    fn default_ci_file_path() -> PathBuf {
        PathBuf::from(".github/workflows/build.yml")
    }
}

#[cfg(test)]
mod test {
    use blue_build_utils::{
        constants::{
            GITHUB_EVENT_NAME, GITHUB_EVENT_PATH, GITHUB_REF_NAME, GITHUB_SHA, PR_EVENT_NUMBER,
        },
        string_vec,
        test_utils::set_env_var,
    };
    use oci_distribution::Reference;
    use rstest::rstest;

    use crate::{
        drivers::{CiDriver, opts::GenerateTagsOpts, types::Platform},
        test::{TEST_TAG_1, TEST_TAG_2, TIMESTAMP},
    };

    use super::GithubDriver;

    const COMMIT_SHA: &str = "1234567";
    const BR_REF_NAME: &str = "feature/test";
    const BR_REF_NAME_CLEAN: &str = "feature_test";

    fn setup_default_branch() {
        setup();
        set_env_var(
            GITHUB_EVENT_PATH,
            "../test-files/github-events/default-branch.json",
        );
        set_env_var(GITHUB_REF_NAME, "main");
    }

    fn setup_pr_branch() {
        setup();
        set_env_var(
            GITHUB_EVENT_PATH,
            "../test-files/github-events/pr-branch.json",
        );
        set_env_var(GITHUB_EVENT_NAME, "pull_request");
        set_env_var(GITHUB_REF_NAME, BR_REF_NAME);
        set_env_var(PR_EVENT_NUMBER, "12");
    }

    fn setup_branch() {
        setup();
        set_env_var(GITHUB_EVENT_PATH, "../test-files/github-events/branch.json");
        set_env_var(GITHUB_REF_NAME, BR_REF_NAME);
    }

    fn setup() {
        set_env_var(GITHUB_EVENT_NAME, "push");
        set_env_var(GITHUB_SHA, "1234567890");
    }

    #[test]
    fn get_registry() {
        setup_default_branch();

        let registry = GithubDriver::get_registry().unwrap();

        assert_eq!(registry, "ghcr.io/test-owner");
    }

    #[test]
    fn on_default_branch_true() {
        setup_default_branch();

        assert!(GithubDriver::on_default_branch());
    }

    #[test]
    fn on_default_branch_false() {
        setup_pr_branch();

        assert!(!GithubDriver::on_default_branch());
    }

    #[test]
    fn get_repo_url() {
        setup_branch();

        let url = GithubDriver::get_repo_url().unwrap();

        assert_eq!(url, "https://example.com/");
    }

    #[rstest]
    #[case::default_branch(
        setup_default_branch,
        None,
        string_vec![
            format!("{}-41", &*TIMESTAMP),
            "latest",
            &*TIMESTAMP,
            format!("{COMMIT_SHA}-41"),
            "41",
        ],
    )]
    #[case::default_branch_alt_tags(
        setup_default_branch,
        Some(bon::vec![TEST_TAG_1, TEST_TAG_2]),
        string_vec![
            TEST_TAG_1,
            format!("{TEST_TAG_1}-41"),
            format!("{}-{TEST_TAG_1}-41", &*TIMESTAMP),
            format!("{COMMIT_SHA}-{TEST_TAG_1}-41"),
            TEST_TAG_2,
            format!("{TEST_TAG_2}-41"),
            format!("{}-{TEST_TAG_2}-41", &*TIMESTAMP),
            format!("{COMMIT_SHA}-{TEST_TAG_2}-41"),
        ],
    )]
    #[case::pr_branch(
        setup_pr_branch,
        None,
        string_vec!["pr-12-41", format!("{COMMIT_SHA}-41")],
    )]
    #[case::pr_branch_alt_tags(
        setup_pr_branch,
        Some(bon::vec![TEST_TAG_1, TEST_TAG_2]),
        string_vec![
            format!("pr-12-{TEST_TAG_1}-41"),
            format!("{COMMIT_SHA}-{TEST_TAG_1}-41"),
            format!("pr-12-{TEST_TAG_2}-41"),
            format!("{COMMIT_SHA}-{TEST_TAG_2}-41"),
        ],
    )]
    #[case::branch(
        setup_branch,
        None,
        string_vec![format!("{COMMIT_SHA}-41"), format!("br-{BR_REF_NAME_CLEAN}-41")],
    )]
    #[case::branch_alt_tags(
        setup_branch,
        Some(bon::vec![TEST_TAG_1, TEST_TAG_2]),
        string_vec![
            format!("br-{BR_REF_NAME_CLEAN}-{TEST_TAG_1}-41"),
            format!("{COMMIT_SHA}-{TEST_TAG_1}-41"),
            format!("br-{BR_REF_NAME_CLEAN}-{TEST_TAG_2}-41"),
            format!("{COMMIT_SHA}-{TEST_TAG_2}-41"),
        ],
    )]
    fn generate_tags(
        #[case] setup: impl FnOnce(),
        #[case] alt_tags: Option<Vec<String>>,
        #[case] mut expected: Vec<String>,
    ) {
        setup();
        expected.sort();
        let oci_ref: Reference = "ghcr.io/ublue-os/silverblue-main".parse().unwrap();

        let mut tags = GithubDriver::generate_tags(
            GenerateTagsOpts::builder()
                .oci_ref(&oci_ref)
                .maybe_alt_tags(alt_tags.as_deref())
                .platform(Platform::LinuxAmd64)
                .build(),
        )
        .unwrap();
        tags.sort();

        assert_eq!(tags, expected);
    }
}
