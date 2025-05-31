use std::path::Path;

use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};
use serde::Deserialize;

use super::ImageRef;

mod private {
    pub trait Private {}
}

impl private::Private for RpmOstreeStatus {}
impl private::Private for BootcStatus {}

pub trait BootStatus: private::Private {
    fn transaction_in_progress(&self) -> bool;
    fn booted_image(&self) -> Option<ImageRef>;
    fn staged_image(&self) -> Option<ImageRef>;
    fn is_booted_on_archive(&self, archive_path: &Path) -> bool;
    fn is_staged_on_archive(&self, archive_path: &Path) -> bool;
}

#[derive(Deserialize, Debug, Clone)]
pub struct BootcStatus {
    status: BootcStatusExt,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct BootcStatusExt {
    staged: Option<BootcStatusImage>,
    booted: BootcStatusImage,
    rollback: Option<BootcStatusImage>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct BootcStatusImage {
    image: BootcStatusImageInfo,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct BootcStatusImageInfo {
    image: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpmOstreeStatus {
    deployments: Vec<RpmOstreeDeployments>,
    transactions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct RpmOstreeDeployments {
    container_image_reference: String,
    booted: bool,
    staged: bool,
}

impl RpmOstreeStatus {
    /// Creates a status struct for `rpm-ostree`.
    ///
    /// # Errors
    /// Errors if the command fails or deserialization fails.
    pub fn try_new() -> Result<Self> {
        blue_build_utils::check_command_exists("rpm-ostree")?;

        trace!("rpm-ostree status --json");
        let output = cmd!("rpm-ostree", "status", "--json")
            .output()
            .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to get `rpm-ostree` status!");
        }

        trace!("{}", String::from_utf8_lossy(&output.stdout));

        serde_json::from_slice(&output.stdout).into_diagnostic()
    }
}

impl BootStatus for RpmOstreeStatus {
    /// Checks if there is a transaction in progress.
    fn transaction_in_progress(&self) -> bool {
        self.transactions.as_ref().is_some_and(|tr| !tr.is_empty())
    }

    /// Get the booted image's reference.
    fn booted_image(&self) -> Option<ImageRef> {
        Some(
            self.deployments
                .iter()
                .find(|deployment| deployment.booted)?
                .container_image_reference
                .to_string(),
        )
    }

    /// Get the booted image's reference.
    fn staged_image(&self) -> Option<String> {
        Some(
            self.deployments
                .iter()
                .find(|deployment| deployment.staged)?
                .container_image_reference
                .to_string(),
        )
    }

    fn is_booted_on_archive(&self, archive_path: &Path) -> bool {
        self.booted_image().is_some_and(|deployment| {
            deployment
                .split(':')
                .next_back()
                .is_some_and(|boot_ref| Path::new(boot_ref) == archive_path)
        })
    }

    fn is_staged_on_archive(&self, archive_path: &Path) -> bool {
        self.staged_image().is_some_and(|deployment| {
            deployment
                .split(':')
                .next_back()
                .is_some_and(|boot_ref| Path::new(boot_ref) == archive_path)
        })
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use blue_build_utils::constants::{
        ARCHIVE_SUFFIX, LOCAL_BUILD, OCI_ARCHIVE, OSTREE_IMAGE_SIGNED, OSTREE_UNVERIFIED_IMAGE,
    };

    use crate::drivers::types::BootStatus;

    use super::{RpmOstreeDeployments, RpmOstreeStatus};

    fn create_image_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .into(),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .into(),
                    booted: false,
                    staged: false,
                },
            ]
            .into(),
            transactions: None,
        }
    }

    fn create_transaction_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .into(),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .into(),
                    booted: false,
                    staged: false,
                },
            ]
            .into(),
            transactions: Some(vec!["Upgrade".into(), "/".into()].into()),
        }
    }

    fn create_archive_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}").into(),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last").into(),
                    booted: false,
                    staged: false,
                },
            ]
            .into(),
            transactions: None,
        }
    }

    fn create_archive_staged_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}").into(),
                    booted: false,
                    staged: true,
                },
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}").into(),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference:
                        format!("{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last").into(),
                    booted: false,
                    staged: false,
                },
            ]
            .into(),
            transactions: None,
        }
    }

    #[test]
    fn test_booted_image() {
        assert!(
            create_image_status()
                .booted_image()
                .expect("Contains image")
                .ends_with("cli/test")
        );
    }

    #[test]
    fn test_staged_image() {
        assert!(
            create_archive_staged_status()
                .staged_image()
                .expect("Contains image")
                .ends_with(&format!("cli_test.{ARCHIVE_SUFFIX}"))
        );
    }

    #[test]
    fn test_transaction_in_progress() {
        assert!(create_transaction_status().transaction_in_progress());
        assert!(!create_image_status().transaction_in_progress());
    }

    #[test]
    fn test_is_booted_archive() {
        assert!(
            !create_archive_status().is_booted_on_archive(
                &Path::new(LOCAL_BUILD).join(format!("cli.{ARCHIVE_SUFFIX}"))
            )
        );
        assert!(create_archive_status().is_booted_on_archive(
            &Path::new(LOCAL_BUILD).join(format!("cli_test.{ARCHIVE_SUFFIX}"))
        ));
    }

    #[test]
    fn test_is_staged_archive() {
        assert!(
            !create_archive_staged_status().is_staged_on_archive(
                &Path::new(LOCAL_BUILD).join(format!("cli.{ARCHIVE_SUFFIX}"))
            )
        );
        assert!(create_archive_staged_status().is_staged_on_archive(
            &Path::new(LOCAL_BUILD).join(format!("cli_test.{ARCHIVE_SUFFIX}"))
        ));
    }
}
