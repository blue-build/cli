use blue_build_utils::credentials::Credentials;
use cached::cached;
use log::{debug, trace};
use miette::{Context, IntoDiagnostic, Result};
use oci_client::{Reference, client::ClientConfig, manifest::OciManifest, secrets::RegistryAuth};

use crate::{
    ASYNC_RUNTIME,
    drivers::{InspectDriver, types::ImageMetadata},
};

use super::opts::GetMetadataOpts;

pub struct OciClientDriver;

impl InspectDriver for OciClientDriver {
    fn get_metadata(opts: GetMetadataOpts) -> Result<ImageMetadata> {
        #[cached(key = "String", convert = r"{image.to_string()}")]
        fn inner(image: &Reference) -> Result<ImageMetadata> {
            let client = oci_client::Client::new(ClientConfig::default());
            let auth = match Credentials::get(image.registry()) {
                Some(Credentials::Basic { username, password }) => {
                    debug!("Using basic auth");
                    RegistryAuth::Basic(username, password.value().into())
                }
                Some(Credentials::Token(token)) => {
                    debug!("Using bearer token");
                    RegistryAuth::Bearer(token.value().into())
                }
                None => {
                    debug!("No auth");
                    RegistryAuth::Anonymous
                }
            };

            let (manifest, digest) = ASYNC_RUNTIME
                .block_on(client.pull_manifest(image, &auth))
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to pull the manifest for {image}"))?;
            debug!("Found OciManifest for {image}");
            trace!("digest: {digest}");
            trace!("{manifest:#?}");

            let manifest_digests = match &manifest {
                OciManifest::Image(manifest) => {
                    debug!("Found single image manifest for {image}");
                    trace!("{manifest}");
                    vec![&digest]
                }
                OciManifest::ImageIndex(index) => {
                    debug!("Found image index for {image}");
                    trace!("{index}");
                    index.manifests.iter().map(|entry| &entry.digest).collect()
                }
            };

            trace!("Found digests: {manifest_digests:#?}");

            let configs = manifest_digests
                .into_iter()
                .map(|digest| {
                    let image = &image.clone_with_digest(digest.clone());
                    let (image_manifest, image_manifest_digest) = ASYNC_RUNTIME
                        .block_on(client.pull_image_manifest(image, &auth))
                        .into_diagnostic()
                        .wrap_err_with(|| format!("Failed to pull image manifest for {image}"))?;
                    debug!("Pulled image manifest for {image}");
                    trace!("digest: {image_manifest_digest}");
                    trace!("{image_manifest:#?}");

                    let config = {
                        let capacity = image_manifest.config.size;
                        let mut c: Vec<u8> = Vec::with_capacity(
                            capacity.try_into().into_diagnostic().wrap_err_with(|| {
                                format!(
                                    concat!(
                                        "Size of image {image} config ",
                                        "({capacity}) could not be converted to usize"
                                    ),
                                    image = image,
                                    capacity = capacity
                                )
                            })?,
                        );
                        ASYNC_RUNTIME
                            .block_on(client.pull_blob(image, &image_manifest.config, &mut c))
                            .into_diagnostic()
                            .wrap_err_with(|| format!("Failed to pull blob for {image}"))?;
                        c
                    };
                    Ok((
                        image_manifest.config.digest,
                        serde_json::from_slice(&config)
                            .inspect(|config| trace!("{config:#?}"))
                            .into_diagnostic()
                            .wrap_err_with(|| {
                                format!(
                                    "Failed to convert config for {image} to ImageConfig:\n{}",
                                    String::from_utf8_lossy(&config)
                                )
                            })?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            debug!("Retrieved configs for {image}");

            trace!(
                "Config digests: {:#?}",
                configs.iter().map(|(digest, _)| digest)
            );

            Ok(ImageMetadata::builder()
                .manifest(manifest)
                .digest(digest)
                .configs(configs)
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
