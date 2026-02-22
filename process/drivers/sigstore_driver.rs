use std::{collections::BTreeMap, fs, path::Path};

use crate::{
    ASYNC_RUNTIME,
    drivers::opts::{PrivateKey, PrivateKeyContents, VerifyType},
};

use super::{
    SigningDriver,
    opts::{CheckKeyPairOpts, GenerateKeyPairOpts, SignOpts, VerifyOpts},
};
use blue_build_utils::{
    BUILD_ID,
    constants::{BUILD_ID_LABEL, COSIGN_PASSWORD, COSIGN_PRIV_PATH, COSIGN_PUB_PATH},
    credentials::Credentials,
    retry,
};
use log::{debug, info, trace};
use miette::{Context, IntoDiagnostic, Result, bail, miette};
use sigstore::{
    cosign::{
        ClientBuilder, Constraint, CosignCapabilities, SignatureLayer,
        constraint::PrivateKeySigner,
        verification_constraint::{PublicKeyVerifier, VerificationConstraintVec},
    },
    crypto::{SigningScheme, signing_key::SigStoreKeyPair},
    errors::SigstoreVerifyConstraintsError,
    registry::{Auth, OciReference},
};
use zeroize::Zeroizing;

pub struct SigstoreDriver;

impl SigningDriver for SigstoreDriver {
    fn generate_key_pair(opts: GenerateKeyPairOpts) -> Result<()> {
        let path = opts.dir.unwrap_or_else(|| Path::new("."));
        let priv_key_path = path.join(COSIGN_PRIV_PATH);
        let pub_key_path = path.join(COSIGN_PUB_PATH);

        if priv_key_path.exists() {
            bail!("Private key file already exists at {COSIGN_PRIV_PATH}");
        } else if pub_key_path.exists() {
            bail!("Public key file already exists at {COSIGN_PUB_PATH}");
        }

        let signer = SigningScheme::default()
            .create_signer()
            .into_diagnostic()
            .context("Failed to create signer")?;

        let keypair = signer
            .to_sigstore_keypair()
            .into_diagnostic()
            .context("Failed to create key pair")?;

        let priv_key = keypair
            .private_key_to_encrypted_pem(b"")
            .into_diagnostic()
            .context("Failed to create encrypted private key")?;
        let pub_key = keypair.public_key_to_pem().into_diagnostic()?;

        fs::write(priv_key_path, priv_key)
            .into_diagnostic()
            .with_context(|| format!("Failed to write {COSIGN_PRIV_PATH}"))?;
        fs::write(pub_key_path, pub_key)
            .into_diagnostic()
            .with_context(|| format!("Failed to write {COSIGN_PUB_PATH}"))?;

        Ok(())
    }

    fn check_signing_files(opts: CheckKeyPairOpts) -> Result<()> {
        trace!("SigstoreDriver::check_signing_files({opts:?})");

        let path = opts.dir.unwrap_or_else(|| Path::new("."));
        let pub_path = path.join(COSIGN_PUB_PATH);

        let pub_key = fs::read_to_string(&pub_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open public key file {}", pub_path.display()))?;
        debug!("Retrieved public key from {COSIGN_PUB_PATH}");
        trace!("{pub_key}");

        let key: Zeroizing<Vec<u8>> = PrivateKey::new(path)
            .context("Failed to get private key")?
            .contents()?;
        debug!("Retrieved private key");

        let keypair = SigStoreKeyPair::from_encrypted_pem(
            &key,
            blue_build_utils::get_env_var(COSIGN_PASSWORD)
                .unwrap_or_default()
                .as_bytes(),
        )
        .into_diagnostic()
        .context("Failed to generate key pair from private key")?;
        let gen_pub = keypair
            .public_key_to_pem()
            .into_diagnostic()
            .context("Failed to generate public key from private key")?;
        debug!("Generated public key from private key");
        trace!("{gen_pub}");

        if pub_key.trim() == gen_pub.trim() {
            debug!("Public and private key matches");
            Ok(())
        } else {
            bail!("Private and public keys do not match.")
        }
    }

    fn sign(
        SignOpts {
            image,
            metadata,
            key,
        }: SignOpts,
    ) -> Result<()> {
        trace!("SigstoreDriver::sign({image}, {metadata:?})");

        let Some(key) = key else {
            bail!("Private key is required to sign");
        };

        let mut client = ClientBuilder::default().build().into_diagnostic()?;

        let signing_scheme = SigningScheme::default();
        let key: Zeroizing<Vec<u8>> = key.contents()?;
        debug!("Retrieved private key");

        let signer = PrivateKeySigner::new_with_signer(
            SigStoreKeyPair::from_encrypted_pem(
                &key,
                blue_build_utils::get_env_var(COSIGN_PASSWORD)
                    .unwrap_or_default()
                    .as_bytes(),
            )
            .into_diagnostic()?
            .to_sigstore_signer(&signing_scheme)
            .into_diagnostic()?,
        );
        debug!("Created signer");

        let auth = match Credentials::get(image.registry()) {
            Some(Credentials::Basic { username, password }) => {
                Auth::Basic(username, password.value().into())
            }
            _ => Auth::Anonymous,
        };
        debug!("Credentials retrieved");

        let digests = metadata.all_digests();
        trace!("Found digests: {digests:#?}");

        let signature_layers = digests
            .into_iter()
            .map(|digest| {
                let image_ref = OciReference::with_digest(
                    image.registry().to_string(),
                    image.repository().to_string(),
                    digest,
                );
                let (cosign_sig_image, cosign_digest) = ASYNC_RUNTIME
                    .block_on(client.triangulate(&image_ref, &auth))
                    .into_diagnostic()?;
                let mut sig_layer =
                    SignatureLayer::new_unsigned(&image_ref, &cosign_digest).into_diagnostic()?;
                signer.add_constraint(&mut sig_layer).into_diagnostic()?;
                Ok((cosign_sig_image, sig_layer))
            })
            .collect::<Result<Vec<_>>>()?;
        debug!("Created signing layers");

        info!("Pushing signatures");

        for (ref cosign_sig_image, sig_layer) in signature_layers {
            ASYNC_RUNTIME
                .block_on(client.push_signature(
                    Some(BTreeMap::from_iter([(
                        BUILD_ID_LABEL.into(),
                        BUILD_ID.to_string(),
                    )])),
                    &auth,
                    cosign_sig_image,
                    vec![sig_layer],
                ))
                .into_diagnostic()
                .with_context(|| format!("Failed to push signatures for image {image}"))?;
        }
        info!("Successfully pushed signatures");

        Ok(())
    }

    fn verify(opts: VerifyOpts) -> Result<()> {
        let mut client = ClientBuilder::default().build().into_diagnostic()?;

        let image_digest: OciReference = opts.image.to_string().parse().into_diagnostic()?;
        trace!("{image_digest:?}");

        let signing_scheme = SigningScheme::default();

        let pub_key = fs::read_to_string(match &opts.verify_type {
            VerifyType::File(path) => path,
            VerifyType::Keyless { .. } => {
                todo!("Keyless currently not supported for sigstore driver")
            }
        })
        .into_diagnostic()
        .with_context(|| format!("Failed to open public key file {COSIGN_PUB_PATH}"))?;
        debug!("Retrieved public key from {COSIGN_PUB_PATH}");
        trace!("{pub_key}");

        let verifier =
            PublicKeyVerifier::new(pub_key.as_bytes(), &signing_scheme).into_diagnostic()?;
        let verification_constraints: VerificationConstraintVec = vec![Box::new(verifier)];

        debug!("Triangulating image");
        let auth = Auth::Anonymous;
        let (cosign_signature_image, source_image_digest) = retry(2, 5, || {
            ASYNC_RUNTIME
                .block_on(client.triangulate(&image_digest, &auth))
                .into_diagnostic()
                .with_context(|| format!("Failed to triangulate image {image_digest}"))
        })?;
        trace!("{cosign_signature_image}, {source_image_digest}");

        let trusted_layers = retry(2, 5, || {
            ASYNC_RUNTIME
                .block_on(client.trusted_signature_layers(
                    &auth,
                    &source_image_digest,
                    &cosign_signature_image,
                ))
                .into_diagnostic()
        })?;

        sigstore::cosign::verify_constraints(&trusted_layers, verification_constraints.iter())
            .map_err(
                |SigstoreVerifyConstraintsError {
                     unsatisfied_constraints,
                 }| {
                    miette!("Failed to verify for constraints: {unsatisfied_constraints:?}")
                },
            )
    }

    fn signing_login(_server: &str) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
    use tempfile::TempDir;

    use crate::drivers::{
        SigningDriver,
        cosign_driver::CosignDriver,
        opts::{CheckKeyPairOpts, GenerateKeyPairOpts},
    };

    use super::SigstoreDriver;

    #[test]
    fn generate_key_pair() {
        let tempdir = TempDir::new().unwrap();

        SigstoreDriver::generate_key_pair(
            GenerateKeyPairOpts::builder().dir(tempdir.path()).build(),
        )
        .unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        SigstoreDriver::check_signing_files(
            CheckKeyPairOpts::builder().dir(tempdir.path()).build(),
        )
        .unwrap();
    }

    #[test]
    fn check_key_pairs() {
        let path = Path::new("../test-files/keys");

        SigstoreDriver::check_signing_files(CheckKeyPairOpts::builder().dir(path).build()).unwrap();
    }

    #[test]
    fn compatibility() {
        let tempdir = TempDir::new().unwrap();

        SigstoreDriver::generate_key_pair(
            GenerateKeyPairOpts::builder().dir(tempdir.path()).build(),
        )
        .unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        CosignDriver::check_signing_files(CheckKeyPairOpts::builder().dir(tempdir.path()).build())
            .unwrap();
    }
}
