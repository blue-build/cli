use std::{fs, path::Path};

use crate::credentials::Credentials;

use super::{functions::get_private_key, SigningDriver};
use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
use log::{debug, trace};
use miette::{bail, miette, Context, IntoDiagnostic};
use sigstore::{
    cosign::{
        constraint::PrivateKeySigner, ClientBuilder, Constraint, CosignCapabilities, SignatureLayer,
    },
    crypto::{signing_key::SigStoreKeyPair, SigningScheme},
    registry::{Auth, OciReference},
};
use zeroize::Zeroizing;

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

        let signer = SigningScheme::default().create_signer().into_diagnostic()?;

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
        trace!("SigstoreDriver::check_signing_files()");

        let pub_key = fs::read_to_string(COSIGN_PUB_PATH)
            .into_diagnostic()
            .with_context(|| format!("Failed to open public key file {COSIGN_PUB_PATH}"))?;
        debug!("Retrieved public key from {COSIGN_PUB_PATH}");
        trace!("{pub_key}");

        let key =
            get_private_key(|priv_key| priv_key.contents()).context("Failed to get private key")?;
        debug!("Retrieved private key");

        let keypair = SigStoreKeyPair::from_pem(&key).into_diagnostic()?;
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

    fn sign(image_digest: &str, _key_arg: Option<String>) -> miette::Result<()> {
        trace!("SigstoreDriver::sign({image_digest})");

        let mut client = ClientBuilder::default().build().into_diagnostic()?;

        let image_digest: OciReference = image_digest.parse().into_diagnostic()?;
        trace!("{image_digest:?}");

        let signing_scheme = SigningScheme::default();
        let key = get_private_key(|key| key.contents())?;
        debug!("Retrieved private key");

        let signer = PrivateKeySigner::new_with_raw(key, Zeroizing::default(), &signing_scheme)
            .into_diagnostic()?;
        debug!("Created signer");

        let Credentials {
            registry: _,
            username,
            password,
        } = Credentials::get().ok_or_else(|| miette!("Credentials are required for signing"))?;
        let auth = Auth::Basic(username.clone(), password.clone());
        debug!("Credentials retrieved");

        let (cosign_signature_image, source_image_digest) =
            smol::block_on(client.triangulate(&image_digest, &auth))
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
        smol::block_on(client.push_signature(
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

    fn verify(_image_name_tag: &str, _verify_type: super::VerifyType) -> miette::Result<()> {
        todo!()
    }

    fn signing_login() -> miette::Result<()> {
        Ok(())
    }
}
