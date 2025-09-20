use crate::drivers::InspectDriver;

use super::opts::GetMetadataOpts;

pub struct OciClientDriver;

impl InspectDriver for OciClientDriver {
    fn get_metadata(_opts: GetMetadataOpts) -> miette::Result<super::types::ImageMetadata> {
        todo!()
    }
}
