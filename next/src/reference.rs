use std::{ops::Deref, str::FromStr, sync::Arc};

use blue_build_process_management::drivers::{Driver, InspectDriver, opts::GetMetadataOpts};
use miette::{IntoDiagnostic, Result};
use oci_client::Reference;

/// A `Reference` that is pinned to a digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PinnedReference(Arc<Reference>);

impl TryFrom<Reference> for PinnedReference {
    type Error = miette::Error;

    fn try_from(value: Reference) -> std::prelude::v1::Result<Self, Self::Error> {
        #[cached::cached(
            sync_writes = "by_key",
            key = "Reference",
            convert = "{ image.clone() }"
        )]
        fn inner(image: Reference) -> Result<PinnedReference> {
            Ok(if image.digest().is_some() {
                PinnedReference(Arc::new(image))
            } else {
                let inspection =
                    Driver::get_metadata(GetMetadataOpts::builder().image(&image).build())?;

                PinnedReference(Arc::new(
                    image.clone_with_digest(inspection.digest().to_string()),
                ))
            })
        }
        inner(value)
    }
}

impl TryFrom<&str> for PinnedReference {
    type Error = miette::Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        Reference::from_str(value)
            .into_diagnostic()
            .and_then(Self::try_from)
    }
}

impl TryFrom<&String> for PinnedReference {
    type Error = miette::Error;

    fn try_from(value: &String) -> std::prelude::v1::Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&Reference> for PinnedReference {
    type Error = miette::Error;

    fn try_from(value: &Reference) -> std::prelude::v1::Result<Self, Self::Error> {
        Self::try_from(value.clone())
    }
}

impl Deref for PinnedReference {
    type Target = Reference;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for PinnedReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
