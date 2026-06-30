use blue_build_utils::credentials::Credentials;
use cached::proc_macro::cached;
use log::trace;
use miette::{IntoDiagnostic, Result};
use oci_client::{Reference, client::ClientConfig, manifest::OciManifest, secrets::RegistryAuth};

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
            // Speak plain HTTP to loopback registries (localhost, 127.0.0.0/8,
            // ::1 — any port), matching the convention of cosign, containerd,
            // and docker. Everything else stays HTTPS. No configuration needed.
            let protocol = if is_loopback_registry(image.registry()) {
                oci_client::client::ClientProtocol::Http
            } else {
                oci_client::client::ClientProtocol::default()
            };
            let client = oci_client::Client::new(ClientConfig {
                protocol,
                ..ClientConfig::default()
            });
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

            let manifest_digests = match &manifest {
                OciManifest::Image(_) => vec![&digest],
                OciManifest::ImageIndex(index) => {
                    index.manifests.iter().map(|entry| &entry.digest).collect()
                }
            };

            trace!("Found digests: {manifest_digests:#?}");

            let configs = manifest_digests
                .into_iter()
                .map(|digest| {
                    let image = &image.clone_with_digest(digest.clone());
                    let (image_manifest, _) = ASYNC_RUNTIME
                        .block_on(client.pull_image_manifest(image, &auth))
                        .into_diagnostic()?;

                    let config = {
                        let mut c: Vec<u8> = vec![];
                        ASYNC_RUNTIME
                            .block_on(client.pull_blob(image, &image_manifest.config, &mut c))
                            .into_diagnostic()?;
                        c
                    };
                    Ok((
                        image_manifest.config.digest,
                        serde_json::from_slice(&config).into_diagnostic()?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

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

/// Returns `true` when `registry` refers to a loopback host — `localhost`, an
/// IPv4 address in `127.0.0.0/8`, or `::1` — optionally with a `:port` suffix
/// and `[...]` brackets around an IPv6 literal.
///
/// Loopback registries are served over plain HTTP (matching cosign,
/// containerd, and docker); every other registry uses HTTPS.
fn is_loopback_registry(registry: &str) -> bool {
    // Strip an optional `:port` suffix, but leave an unbracketed IPv6 literal
    // (which itself contains `:`) intact.
    let host = registry
        .rsplit_once(':')
        .filter(|(h, _)| !h.contains(':') || h.ends_with(']'))
        .map_or(registry, |(h, _)| h);
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

#[cfg(test)]
mod test {
    use super::is_loopback_registry;

    #[test]
    fn loopback_registries_use_http() {
        for registry in [
            "localhost",
            "localhost:5000",
            "127.0.0.1",
            "127.0.0.1:5000",
            "127.5.6.7", // anywhere in 127.0.0.0/8
            "::1",
            "[::1]",
            "[::1]:5000",
        ] {
            assert!(
                is_loopback_registry(registry),
                "{registry} should be treated as loopback"
            );
        }
    }

    #[test]
    fn remote_registries_use_https() {
        for registry in [
            "ghcr.io",
            "registry.example.com",
            "registry.example.com:5000",
            "192.168.1.10",
            "192.168.1.10:5000",
            "8.8.8.8",
            "[2001:db8::1]:5000",
            "localhost.example.com", // not the loopback host
        ] {
            assert!(
                !is_loopback_registry(registry),
                "{registry} should not be treated as loopback"
            );
        }
    }
}
