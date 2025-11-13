use std::path::PathBuf;

use blue_build_utils::{
    constants::{
        CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
        CI_PIPELINE_SOURCE, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_PROJECT_URL, CI_REGISTRY,
        CI_SERVER_HOST, CI_SERVER_PROTOCOL,
    },
    container::Tag,
    string_vec,
};
use log::trace;

#[cfg(not(test))]
use blue_build_utils::get_env_var;

#[cfg(test)]
use blue_build_utils::test_utils::get_env_var;
use miette::Result;

use crate::drivers::Driver;

use super::{CiDriver, opts::GenerateTagsOpts};

pub struct GitlabDriver;

impl CiDriver for GitlabDriver {
    fn on_default_branch() -> bool {
        get_env_var(CI_DEFAULT_BRANCH).is_ok_and(|default_branch| {
            get_env_var(CI_COMMIT_REF_NAME).is_ok_and(|branch| default_branch == branch)
        })
    }

    fn keyless_cert_identity() -> Result<String> {
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

    fn generate_tags(opts: GenerateTagsOpts) -> Result<Vec<Tag>> {
        const MR_EVENT: &str = "merge_request_event";
        let os_version = Driver::get_os_version().oci_ref(opts.oci_ref).call()?;
        let timestamp = blue_build_utils::get_tag_timestamp();
        let short_sha =
            get_env_var(CI_COMMIT_SHORT_SHA).inspect(|v| trace!("{CI_COMMIT_SHORT_SHA}={v}"))?;
        let ref_name = get_env_var(CI_COMMIT_REF_NAME)
            .inspect(|v| trace!("{CI_COMMIT_REF_NAME}={v}"))?
            .replace('/', "_");

        let tags = match (
            Self::on_default_branch(),
            opts.alt_tags.as_ref(),
            get_env_var(CI_MERGE_REQUEST_IID).inspect(|v| trace!("{CI_MERGE_REQUEST_IID}={v}")),
            get_env_var(CI_PIPELINE_SOURCE).inspect(|v| trace!("{CI_PIPELINE_SOURCE}={v}")),
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
                        alt,
                        format!("{alt}-{os_version}"),
                        format!("{timestamp}-{alt}-{os_version}"),
                        format!("{short_sha}-{alt}-{os_version}"),
                    ]
                })
                .collect(),
            (false, None, Ok(mr_iid), Ok(pipeline_source)) if pipeline_source == MR_EVENT => {
                vec![
                    format!("mr-{mr_iid}-{os_version}"),
                    format!("{short_sha}-{os_version}"),
                ]
            }
            (false, None, _, _) => {
                vec![
                    format!("br-{ref_name}-{os_version}"),
                    format!("{short_sha}-{os_version}"),
                ]
            }
            (false, Some(alt_tags), Ok(mr_iid), Ok(pipeline_source))
                if pipeline_source == MR_EVENT =>
            {
                alt_tags
                    .iter()
                    .flat_map(|alt| {
                        vec![
                            format!("mr-{mr_iid}-{alt}-{os_version}"),
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
        }
        .into_iter()
        .map(|tag| tag.parse())
        .collect::<Result<Vec<Tag>>>()?;
        trace!("{tags:?}");

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

    fn default_ci_file_path() -> PathBuf {
        PathBuf::from(".gitlab-ci.yml")
    }
}

#[cfg(test)]
mod test {
    use blue_build_utils::{
        constants::{
            CI_COMMIT_REF_NAME, CI_COMMIT_SHORT_SHA, CI_DEFAULT_BRANCH, CI_MERGE_REQUEST_IID,
            CI_PIPELINE_SOURCE, CI_PROJECT_NAME, CI_PROJECT_NAMESPACE, CI_REGISTRY, CI_SERVER_HOST,
            CI_SERVER_PROTOCOL,
        },
        container::Tag,
        platform::Platform,
        string_vec,
        test_utils::set_env_var,
    };
    use oci_distribution::Reference;
    use rstest::rstest;

    use crate::{
        drivers::{CiDriver, opts::GenerateTagsOpts},
        test::{TEST_TAG_1, TEST_TAG_2, TIMESTAMP},
    };

    use super::GitlabDriver;

    const COMMIT_SHA: &str = "1234567";
    const BR_REF_NAME: &str = "feature/test";
    const BR_REF_NAME_CLEAN: &str = "feature_test";

    fn setup_default_branch() {
        setup();
        set_env_var(CI_COMMIT_REF_NAME, "main");
    }

    fn setup_mr_branch() {
        setup();
        set_env_var(CI_MERGE_REQUEST_IID, "12");
        set_env_var(CI_PIPELINE_SOURCE, "merge_request_event");
        set_env_var(CI_COMMIT_REF_NAME, BR_REF_NAME);
    }

    fn setup_branch() {
        setup();
        set_env_var(CI_COMMIT_REF_NAME, BR_REF_NAME);
    }

    fn setup() {
        set_env_var(CI_DEFAULT_BRANCH, "main");
        set_env_var(CI_COMMIT_SHORT_SHA, COMMIT_SHA);
        set_env_var(CI_REGISTRY, "registry.example.com");
        set_env_var(CI_PROJECT_NAMESPACE, "test-project");
        set_env_var(CI_PROJECT_NAME, "test");
        set_env_var(CI_SERVER_PROTOCOL, "https");
        set_env_var(CI_SERVER_HOST, "gitlab.example.com");
    }

    #[test]
    fn get_registry() {
        setup();

        let registry = GitlabDriver::get_registry().unwrap();

        assert_eq!(registry, "registry.example.com/test-project/test");
    }

    #[test]
    fn on_default_branch_true() {
        setup_default_branch();

        assert!(GitlabDriver::on_default_branch());
    }

    #[test]
    fn on_default_branch_false() {
        setup_branch();

        assert!(!GitlabDriver::on_default_branch());
    }

    #[test]
    fn get_repo_url() {
        setup();

        let url = GitlabDriver::get_repo_url().unwrap();

        assert_eq!(url, "https://gitlab.example.com/test-project/test");
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
        setup_mr_branch,
        None,
        string_vec!["mr-12-41", format!("{COMMIT_SHA}-41")],
    )]
    #[case::pr_branch_alt_tags(
        setup_mr_branch,
        Some(bon::vec![TEST_TAG_1, TEST_TAG_2]),
        string_vec![
            format!("mr-12-{TEST_TAG_1}-41"),
            format!("{COMMIT_SHA}-{TEST_TAG_1}-41"),
            format!("mr-12-{TEST_TAG_2}-41"),
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
        let expected: Vec<Tag> = expected
            .into_iter()
            .map(|tag| tag.parse().unwrap())
            .collect();
        let oci_ref: Reference = "ghcr.io/ublue-os/silverblue-main".parse().unwrap();
        let alt_tags = alt_tags.map(|tags| {
            tags.into_iter()
                .map(|tag| tag.parse::<Tag>().unwrap())
                .collect::<Vec<_>>()
        });

        let mut tags = GitlabDriver::generate_tags(
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
