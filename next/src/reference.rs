use std::{ops::Deref, sync::Arc};

use blue_build_process_management::drivers::{Driver, InspectDriver, opts::GetMetadataOpts};
use miette::Result;
use oci_client::Reference;

/// A `Reference` that is pinned to a digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PinnedReference(Arc<Reference>);

impl PinnedReference {
    /// Pins a `Reference` to a digest
    /// by inspecting the registry.
    ///
    /// # Errors
    /// Will error if the call to the registry fails.
    pub async fn pin_image(image: Reference) -> Result<Self> {
        Ok(if image.digest().is_some() {
            Self(Arc::new(image))
        } else {
            let inspection =
                Driver::get_metadata(GetMetadataOpts::builder().image(&image).build()).await?;

            Self(Arc::new(
                image.clone_with_digest(inspection.digest().to_string()),
            ))
        })
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
