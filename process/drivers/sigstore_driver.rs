use std::{fs, path::Path};

use crate::{
    drivers::opts::{PrivateKeyContents, VerifyType},
    RT,
};

use super::{
    functions::get_private_key,
    opts::{CheckKeyPairOpts, GenerateKeyPairOpts, SignOpts, VerifyOpts},
    SigningDriver,
};
use blue_build_utils::{
    constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH},
    credentials::Credentials,
};
use log::{debug, trace};
use miette::{bail, miette, Context, IntoDiagnostic};
use sigstore::{
    cosign::{
        constraint::PrivateKeySigner,
        verification_constraint::{PublicKeyVerifier, VerificationConstraintVec},
        ClientBuilder, Constraint, CosignCapabilities, SignatureLayer,
    },
    crypto::{signing_key::SigStoreKeyPair, SigningScheme},
    errors::SigstoreVerifyConstraintsError,
    registry::{Auth, OciReference},
};
use zeroize::Zeroizing;

pub struct SigstoreDriver;

impl SigningDriver for SigstoreDriver {
    fn generate_key_pair(opts: &GenerateKeyPairOpts) -> miette::Result<()> {
        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);
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

    fn check_signing_files(opts: &CheckKeyPairOpts) -> miette::Result<()> {
        trace!("SigstoreDriver::check_signing_files({opts:?})");

        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);
        let pub_path = path.join(COSIGN_PUB_PATH);

        let pub_key = fs::read_to_string(&pub_path)
            .into_diagnostic()
            .with_context(|| format!("Failed to open public key file {}", pub_path.display()))?;
        debug!("Retrieved public key from {COSIGN_PUB_PATH}");
        trace!("{pub_key}");

        let key: Zeroizing<String> = get_private_key(path)
            .context("Failed to get private key")?
            .contents()?;
        debug!("Retrieved private key");

        let keypair = SigStoreKeyPair::from_encrypted_pem(key.as_bytes(), b"")
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

    fn sign(opts: &SignOpts) -> miette::Result<()> {
        trace!("SigstoreDriver::sign({opts:?})");

        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);
        let mut client = ClientBuilder::default().build().into_diagnostic()?;

        let image_digest: &str = opts.image.as_ref();
        let image_digest: OciReference = image_digest.parse().into_diagnostic()?;
        trace!("{image_digest:?}");

        let signing_scheme = SigningScheme::default();
        let key: Zeroizing<Vec<u8>> = get_private_key(path)?.contents()?;
        debug!("Retrieved private key");

        let signer = PrivateKeySigner::new_with_signer(
            SigStoreKeyPair::from_encrypted_pem(&key, b"")
                .into_diagnostic()?
                .to_sigstore_signer(&signing_scheme)
                .into_diagnostic()?,
        );
        debug!("Created signer");

        let Credentials {
            registry: _,
            username,
            password,
        } = Credentials::get().ok_or_else(|| miette!("Credentials are required for signing"))?;
        let auth = Auth::Basic(username.clone(), password.clone());
        debug!("Credentials retrieved");

        let (cosign_signature_image, source_image_digest) = RT
            .block_on(client.triangulate(&image_digest, &auth))
            .into_diagnostic()
            .with_context(|| format!("Failed to triangulate image {image_digest}"))?;
        debug!("Triangulating image");
        trace!("{cosign_signature_image}, {source_image_digest}");

        let mut signature_layer =
            SignatureLayer::new_unsigned(&image_digest, &source_image_digest).into_diagnostic()?;
        signer
            .add_constraint(&mut signature_layer)
            .into_diagnostic()?;
        debug!("Created signing layer");

        debug!("Pushing signature");
        RT.block_on(client.push_signature(
            None,
            &auth,
            &cosign_signature_image,
            vec![signature_layer],
        ))
        .into_diagnostic()
        .with_context(|| {
            format!("Failed to push signature {cosign_signature_image} for image {image_digest}")
        })?;
        debug!("Successfully pushed signature");

        Ok(())
    }

    fn verify(opts: &VerifyOpts) -> miette::Result<()> {
        let mut client = ClientBuilder::default().build().into_diagnostic()?;

        let image_digest: &str = opts.image.as_ref();
        let image_digest: OciReference = image_digest.parse().into_diagnostic()?;
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

        let auth = Auth::Anonymous;
        let (cosign_signature_image, source_image_digest) = RT
            .block_on(client.triangulate(&image_digest, &auth))
            .into_diagnostic()
            .with_context(|| format!("Failed to triangulate image {image_digest}"))?;
        debug!("Triangulating image");
        trace!("{cosign_signature_image}, {source_image_digest}");

        let trusted_layers = RT
            .block_on(client.trusted_signature_layers(
                &auth,
                &source_image_digest,
                &cosign_signature_image,
            ))
            .into_diagnostic()?;

        sigstore::cosign::verify_constraints(&trusted_layers, verification_constraints.iter())
            .map_err(
                |SigstoreVerifyConstraintsError {
                     unsatisfied_constraints,
                 }| {
                    miette!("Failed to verify for constraints: {unsatisfied_constraints:?}")
                },
            )
    }

    fn signing_login() -> miette::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
    use tempdir::TempDir;

    use crate::drivers::{
        cosign_driver::CosignDriver,
        opts::{CheckKeyPairOpts, GenerateKeyPairOpts},
        SigningDriver,
    };

    use super::SigstoreDriver;

    #[test]
    fn generate_key_pair() {
        let tempdir = TempDir::new("keypair").unwrap();

        let gen_opts = GenerateKeyPairOpts::builder().dir(tempdir.path()).build();

        SigstoreDriver::generate_key_pair(&gen_opts).unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        let check_opts = CheckKeyPairOpts::builder().dir(tempdir.path()).build();

        SigstoreDriver::check_signing_files(&check_opts).unwrap();
    }

    #[test]
    fn check_key_pairs() {
        let path = Path::new("../test-files/keys");

        let opts = CheckKeyPairOpts::builder().dir(path).build();

        SigstoreDriver::check_signing_files(&opts).unwrap();
    }

    #[test]
    fn compatibility() {
        let tempdir = TempDir::new("keypair").unwrap();

        let gen_opts = GenerateKeyPairOpts::builder().dir(tempdir.path()).build();

        SigstoreDriver::generate_key_pair(&gen_opts).unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        let check_opts = CheckKeyPairOpts::builder().dir(tempdir.path()).build();

        CosignDriver::check_signing_files(&check_opts).unwrap();
    }
}
