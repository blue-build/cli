use std::{ops::Deref, sync::Arc};

use blue_build_process_management::drivers::{Driver, InspectDriver, opts::GetMetadataOpts};
use miette::Result;
use oci_client::Reference;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PinnedReference(Reference);

impl PinnedReference {
    /// Pins a `Reference` to a digest and caches
    /// the result for the life of the program.
    ///
    /// # Errors
    /// Will error if the image cannot be found
    /// or if unauthorized.
    pub fn pin_image(image: &Reference) -> Result<Arc<Self>> {
        #[cached::cached(
            sync_writes = "by_key",
            key = "Reference",
            convert = "{ image.clone() }"
        )]
        fn inner(image: &Reference) -> Result<Arc<PinnedReference>> {
            Ok(Arc::new(if image.digest().is_some() {
                PinnedReference(image.clone())
            } else {
                let inspection =
                    Driver::get_metadata(GetMetadataOpts::builder().image(image).build())?;

                PinnedReference(image.clone_with_digest(inspection.digest().to_string()))
            }))
        }
        inner(image)
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
