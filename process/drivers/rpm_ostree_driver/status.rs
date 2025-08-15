use blue_build_utils::container::ImageRef;
use image_ref::DeploymentImageRef;
use serde::Deserialize;

use crate::drivers::BootStatus;

mod image_ref;

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    deployments: Vec<Deployment>,
    transactions: Option<Vec<String>>,
}

impl BootStatus for Status {
    /// Checks if there is a transaction in progress.
    fn transaction_in_progress(&self) -> bool {
        self.transactions.as_ref().is_some_and(|tr| !tr.is_empty())
    }

    /// Get the booted image's reference.
    fn booted_image(&self) -> Option<ImageRef<'_>> {
        (&self
            .deployments
            .iter()
            .find(|deployment| deployment.booted)?
            .container_image_reference)
            .try_into()
            .inspect_err(|e| {
                log::warn!("{e}");
            })
            .ok()
    }

    /// Get the booted image's reference.
    fn staged_image(&self) -> Option<ImageRef<'_>> {
        (&self
            .deployments
            .iter()
            .find(|deployment| deployment.staged)?
            .container_image_reference)
            .try_into()
            .inspect_err(|e| {
                log::warn!("{e}");
            })
            .ok()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Deployment {
    container_image_reference: DeploymentImageRef,
    booted: bool,
    staged: bool,
}

#[cfg(test)]
mod test {
    use blue_build_utils::{
        constants::{
            ARCHIVE_SUFFIX, LOCAL_BUILD, OCI_ARCHIVE, OSTREE_IMAGE_SIGNED, OSTREE_UNVERIFIED_IMAGE,
        },
        container::ImageRef,
    };

    use crate::drivers::BootStatus;

    use super::{Deployment, Status};

    fn create_image_status() -> Status {
        Status {
            deployments: vec![
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .try_into()
                    .unwrap(),
                    booted: true,
                    staged: false,
                },
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .try_into()
                    .unwrap(),
                    booted: false,
                    staged: false,
                },
            ],
            transactions: None,
        }
    }

    fn create_transaction_status() -> Status {
        Status {
            deployments: vec![
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    )
                    .try_into()
                    .unwrap(),
                    booted: true,
                    staged: false,
                },
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    )
                    .try_into()
                    .unwrap(),
                    booted: false,
                    staged: false,
                },
            ],
            transactions: Some(bon::vec!["Upgrade", "/"]),
        }
    }

    fn create_archive_staged_status() -> Status {
        Status {
            deployments: vec![
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}"
                    ).try_into().unwrap(),
                    booted: false,
                    staged: true,
                },
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}"
                    ).try_into().unwrap(),
                    booted: true,
                    staged: false,
                },
                Deployment {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    ).try_into().unwrap(),
                    booted: false,
                    staged: false,
                },
            ],
            transactions: None,
        }
    }

    #[test]
    fn test_booted_image() {
        assert!(matches!(
            create_image_status()
                .booted_image()
                .expect("Contains image"),
            ImageRef::Remote(_)
        ));
    }

    #[test]
    fn test_staged_image() {
        assert!(matches!(
            create_archive_staged_status()
                .staged_image()
                .expect("Contains image"),
            ImageRef::LocalTar(_)
        ));
    }

    #[test]
    fn test_transaction_in_progress() {
        assert!(create_transaction_status().transaction_in_progress());
        assert!(!create_image_status().transaction_in_progress());
    }
}
