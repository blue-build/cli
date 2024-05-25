use std::{borrow::Cow, path::Path, process::Command};

use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RpmOstreeStatus<'a> {
    deployments: Cow<'a, [RpmOstreeDeployments<'a>]>,
    transactions: Option<Cow<'a, [Cow<'a, str>]>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RpmOstreeDeployments<'a> {
    container_image_reference: Cow<'a, str>,
    booted: bool,
}

impl<'a> RpmOstreeStatus<'a> {
    /// Creates a status struct for `rpm-ostree`.
    ///
    /// # Errors
    /// Errors if the command fails or deserialization fails.
    pub fn try_new() -> Result<Self> {
        blue_build_utils::check_command_exists("rpm-ostree")?;
        let output = Command::new("rpm-ostree")
            .args(["status", "--json"])
            .output()?;

        if !output.status.success() {
            bail!("Failed to get `rpm-ostree` status!");
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Checks if there is a transaction in progress.
    #[must_use]
    pub fn transaction_in_progress(&self) -> bool {
        self.transactions.as_ref().is_some_and(|tr| !tr.is_empty())
    }

    /// Get the booted image's reference.
    #[must_use]
    pub fn booted_image(&self) -> Option<String> {
        Some(
            self.deployments
                .iter()
                .find(|deployment| deployment.booted)?
                .container_image_reference
                .to_string(),
        )
    }

    #[must_use]
    pub fn is_booted_on_archive<P>(&self, archive_path: P) -> bool
    where
        P: AsRef<Path>,
    {
        self.booted_image().is_some_and(|deployment| {
            deployment
                .split(':')
                .last()
                .is_some_and(|boot_ref| Path::new(boot_ref) == archive_path.as_ref())
        })
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use blue_build_utils::constants::{
        ARCHIVE_SUFFIX, LOCAL_BUILD, OCI_ARCHIVE, OSTREE_IMAGE_SIGNED, OSTREE_UNVERIFIED_IMAGE,
    };

    use super::{RpmOstreeDeployments, RpmOstreeStatus};

    fn create_image_status<'a>() -> RpmOstreeStatus<'a> {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .into(),
                    booted: true,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .into(),
                    booted: false,
                },
            ]
            .into(),
            transactions: None,
        }
    }

    fn create_transaction_status<'a>() -> RpmOstreeStatus<'a> {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .into(),
                    booted: true,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .into(),
                    booted: false,
                },
            ]
            .into(),
            transactions: Some(vec!["Upgrade".into(), "/".into()].into()),
        }
    }

    fn create_archive_status<'a>() -> RpmOstreeStatus<'a> {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}").into(),
                    booted: true,
                },
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last").into(),
                    booted: false,
                },
            ]
            .into(),
            transactions: None,
        }
    }

    #[test]
    fn test_booted_image() {
        assert!(create_image_status()
            .booted_image()
            .expect("Contains image")
            .ends_with("cli/test"));
    }

    #[test]
    fn test_transaction_in_progress() {
        assert!(create_transaction_status().transaction_in_progress());
        assert!(!create_image_status().transaction_in_progress());
    }

    #[test]
    fn test_is_booted_archive() {
        assert!(!create_archive_status()
            .is_booted_on_archive(Path::new(LOCAL_BUILD).join(format!("cli.{ARCHIVE_SUFFIX}"))));
        assert!(create_archive_status().is_booted_on_archive(
            Path::new(LOCAL_BUILD).join(format!("cli_test.{ARCHIVE_SUFFIX}"))
        ));
    }
}
