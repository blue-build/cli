use std::{ops::Not, path::PathBuf};

use blue_build_utils::constants::{OCI_ARCHIVE, OSTREE_UNVERIFIED_IMAGE};
use comlexr::cmd;
use log::trace;
use miette::{Context, IntoDiagnostic, bail};
use oci_distribution::Reference;
use serde::Deserialize;

use crate::logging::CommandLogging;

use super::{BootDriver, BootStatus, opts::SwitchOpts, types::ImageRef};

pub struct RpmOstreeDriver;

impl BootDriver for RpmOstreeDriver {
    fn status() -> miette::Result<Box<dyn BootStatus>> {
        let output = {
            let c = cmd!("rpm-ostree", "status", "--json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        if !output.status.success() {
            bail!("Failed to get `rpm-ostree` status!");
        }

        trace!("{}", String::from_utf8_lossy(&output.stdout));

        Ok(Box::new(
            serde_json::from_slice::<RpmOstreeStatus>(&output.stdout)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Failed to deserialize rpm-ostree status:\n{}",
                        String::from_utf8_lossy(&output.stdout)
                    )
                })?,
        ))
    }

    fn switch(opts: SwitchOpts) -> miette::Result<()> {
        let status = {
            let c = cmd!(
                "rpm-ostree",
                "rebase",
                format!("{OSTREE_UNVERIFIED_IMAGE}:containers-storage:{}", opts.image),
                if opts.reboot => "--reboot",
            );

            trace!("{c:?}");
            c
        }
        .build_status(format!("{}", opts.image), "Switching to new image")
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to image {}", opts.image);
        }

        Ok(())
    }

    fn upgrade(opts: SwitchOpts) -> miette::Result<()> {
        let status = {
            let c = cmd!(
                "rpm-ostree",
                "upgrade",
                if opts.reboot => "--reboot",
            );

            trace!("{c:?}");
            c
        }
        .build_status(format!("{}", opts.image), "Switching to new image")
        .into_diagnostic()?;

        if status.success().not() {
            bail!("Failed to switch to image {}", opts.image);
        }

        Ok(())
    }
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

impl BootStatus for RpmOstreeStatus {
    /// Checks if there is a transaction in progress.
    fn transaction_in_progress(&self) -> bool {
        self.transactions.as_ref().is_some_and(|tr| !tr.is_empty())
    }

    /// Get the booted image's reference.
    fn booted_image(&self) -> Option<ImageRef> {
        Some(image_ref(
            &self
                .deployments
                .iter()
                .find(|deployment| deployment.booted)?
                .container_image_reference,
        ))
    }

    /// Get the booted image's reference.
    fn staged_image(&self) -> Option<ImageRef> {
        Some(image_ref(
            &self
                .deployments
                .iter()
                .find(|deployment| deployment.staged)?
                .container_image_reference,
        ))
    }
}

fn image_ref(image: &str) -> ImageRef {
    match Reference::try_from(image) {
        Ok(reference) => reference.into(),
        _ if image.contains(&format!("{OCI_ARCHIVE}:")) => PathBuf::from(
            image
                .split(':')
                .next_back()
                .expect("Should have at lease one colon"),
        )
        .into(),
        _ => ImageRef::Other(image.into()),
    }
}

#[cfg(test)]
mod test {
    use blue_build_utils::constants::{
        ARCHIVE_SUFFIX, LOCAL_BUILD, OCI_ARCHIVE, OSTREE_IMAGE_SIGNED, OSTREE_UNVERIFIED_IMAGE,
    };

    use crate::drivers::{BootStatus, types::ImageRef};

    use super::{RpmOstreeDeployments, RpmOstreeStatus};

    fn create_image_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    ),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    ),
                    booted: false,
                    staged: false,
                },
            ],
            transactions: None,
        }
    }

    fn create_transaction_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test"
                    ),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    ),
                    booted: false,
                    staged: false,
                },
            ],
            transactions: Some(bon::vec!["Upgrade", "/"]),
        }
    }

    fn create_archive_staged_status() -> RpmOstreeStatus {
        RpmOstreeStatus {
            deployments: vec![
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}"
                    ),
                    booted: false,
                    staged: true,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_UNVERIFIED_IMAGE}:{OCI_ARCHIVE}:{LOCAL_BUILD}/cli_test.{ARCHIVE_SUFFIX}"
                    ),
                    booted: true,
                    staged: false,
                },
                RpmOstreeDeployments {
                    container_image_reference: format!(
                        "{OSTREE_IMAGE_SIGNED}:docker://ghcr.io/blue-build/cli/test:last"
                    ),
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
