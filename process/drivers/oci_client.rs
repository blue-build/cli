use blue_build_utils::credentials::Credentials;
use cached::proc_macro::cached;
use log::trace;
use miette::{IntoDiagnostic, Result};
use oci_distribution::{Reference, client::ClientConfig, secrets::RegistryAuth};

use crate::{
    ASYNC_RUNTIME,
    drivers::{InspectDriver, types::ImageMetadata},
};

use super::opts::GetMetadataOpts;

pub struct OciClientDriver;

impl InspectDriver for OciClientDriver {
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata> {
        #[cached(result = true, key = "String", convert = r"{image.to_string()}")]
        fn inner(image: &Reference) -> Result<ImageMetadata> {
            let client = oci_distribution::Client::new(ClientConfig::default());
            let auth = match Credentials::get(image.registry()) {
                Some(Credentials::Basic { username, password }) => {
                    RegistryAuth::Basic(username, password.value().into())
                }
                Some(Credentials::Token(token)) => RegistryAuth::Bearer(token.value().into()),
                None => RegistryAuth::Anonymous,
            };

            let (manifest, digest) = ASYNC_RUNTIME
                .block_on(client.pull_manifest(image, &auth))
                .into_diagnostic()?;
            let (image_manifest, _image_digest) = ASYNC_RUNTIME
                .block_on(client.pull_image_manifest(image, &auth))
                .into_diagnostic()?;
            let config = {
                let mut c: Vec<u8> = vec![];
                ASYNC_RUNTIME
                    .block_on(client.pull_blob(image, &image_manifest.config, &mut c))
                    .into_diagnostic()?;
                c
            };
            Ok(ImageMetadata::builder()
                .manifest(manifest)
                .digest(digest)
                .config(serde_json::from_slice(&config).into_diagnostic()?)
                .build())
        }
        trace!("OciClientDriver::get_metadata({opts:?})");

        if opts.no_cache {
            inner_prime_cache(opts.image)
        } else {
            inner(opts.image)
        }
    }
}
