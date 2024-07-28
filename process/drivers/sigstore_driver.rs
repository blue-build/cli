use std::{env, fs, path::Path};

use crate::credentials::Credentials;

use super::{
    functions::{get_private_key, PrivateKey},
    SigningDriver,
};
use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
use miette::{bail, miette, Context, IntoDiagnostic};
use sigstore::{
    cosign::{ClientBuilder, CosignCapabilities},
    crypto::{signing_key::SigStoreKeyPair, SigningScheme},
    registry::{Auth, OciReference},
};

pub struct SigstoreDriver;

impl SigningDriver for SigstoreDriver {
    fn generate_key_pair() -> miette::Result<()> {
        let priv_key_path = Path::new(COSIGN_PRIV_PATH);
        let pub_key_path = Path::new(COSIGN_PUB_PATH);

        if priv_key_path.exists() {
            bail!("Private key file already exists at {COSIGN_PRIV_PATH}");
        } else if pub_key_path.exists() {
            bail!("Public key file already exists at {COSIGN_PUB_PATH}");
        }

        let signer = SigningScheme::ECDSA_P256_SHA256_ASN1
            .create_signer()
            .into_diagnostic()?;

        let keypair = signer.to_sigstore_keypair().into_diagnostic()?;

        let priv_key = keypair.private_key_to_pem().into_diagnostic()?;
        let pub_key = keypair.public_key_to_pem().into_diagnostic()?;

        fs::write(priv_key_path, priv_key)
            .into_diagnostic()
            .with_context(|| format!("Failed to write {COSIGN_PRIV_PATH}"))?;
        fs::write(pub_key_path, pub_key)
            .into_diagnostic()
            .with_context(|| format!("Failed to write {COSIGN_PUB_PATH}"))?;

        Ok(())
    }

    fn check_signing_files() -> miette::Result<()> {
        let pub_key = fs::read_to_string(COSIGN_PUB_PATH)
            .into_diagnostic()
            .with_context(|| format!("Failed to open public key file {COSIGN_PUB_PATH}"))?;
        let key = get_private_key(|priv_key| match priv_key {
            PrivateKey::Env(env) => env::var(env).into_diagnostic(),
            PrivateKey::Path(path) => fs::read_to_string(path).into_diagnostic(),
        })
        .context("Failed to get private key")?;

        let keypair = SigStoreKeyPair::from_pem(key.as_bytes()).into_diagnostic()?;
        let gen_pub = keypair
            .public_key_to_pem()
            .into_diagnostic()
            .context("Failed to generate public key from private key")?;

        if pub_key.trim() == gen_pub.trim() {
            Ok(())
        } else {
            bail!("Private and public keys do not match.")
        }
    }

    fn sign(image_digest: &str, _key_arg: Option<String>) -> miette::Result<()> {
        smol::block_on(async {
            let Credentials {
                registry: _,
                username,
                password,
            } = Credentials::get().ok_or(miette!("Credentials are required for signing"))?;

            let auth = Auth::Basic(username.clone(), password.clone());
            let mut client = ClientBuilder::default().build().into_diagnostic()?;
            let image_digest: OciReference = image_digest.parse().into_diagnostic()?;
            let (cosign_signature_image, source_image_digest) = client
                .triangulate(&image_digest, &auth)
                .await
                .into_diagnostic()
                .with_context(|| format!("Failed to triangulate image {image_digest}"))?;

            Ok(())
        })
    }

    fn verify(_image_name_tag: &str, _verify_type: super::VerifyType) -> miette::Result<()> {
        todo!()
    }

    fn signing_login() -> miette::Result<()> {
        todo!()
    }
}
