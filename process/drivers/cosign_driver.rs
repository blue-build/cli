use std::{fmt::Debug, fs, process::Command};

use blue_build_utils::constants::{COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES};
use log::{debug, trace};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::credentials::Credentials;

use super::SigningDriver;

#[derive(Debug)]
pub struct CosignDriver;

impl SigningDriver for CosignDriver {
    fn generate_key_pair() -> Result<()> {
        let status = Command::new("cosign")
            .env(COSIGN_PASSWORD, "")
            .env(COSIGN_YES, "true")
            .arg("genereate-key-pair")
            .status()
            .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to generate cosign key-pair!");
        }

        Ok(())
    }

    fn check_signing_files() -> Result<()> {
        super::get_private_key(|priv_key| {
            trace!("cosign public-key --key {priv_key}");
            let output = Command::new("cosign")
                .env(COSIGN_PASSWORD, "")
                .env(COSIGN_YES, "true")
                .arg("public-key")
                .arg(format!("--key={priv_key}"))
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                bail!(
                    "Failed to run cosign public-key: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            let calculated_pub_key = String::from_utf8(output.stdout).into_diagnostic()?;
            let found_pub_key = fs::read_to_string(COSIGN_PUB_PATH)
                .into_diagnostic()
                .with_context(|| format!("Failed to read {COSIGN_PUB_PATH}"))?;
            trace!("calculated_pub_key={calculated_pub_key},found_pub_key={found_pub_key}");

            if calculated_pub_key.trim() == found_pub_key.trim() {
                debug!("Cosign files match, continuing build");
                Ok(())
            } else {
                bail!("Public key '{COSIGN_PUB_PATH}' does not match private key")
            }
        })
    }

    fn sign_images<S, T>(image_name: S, tag: Option<T>) -> Result<()>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        super::sign_images(image_name, tag, Self::sign, Self::verify)
    }

    fn signing_login() -> Result<()> {
        trace!("DockerDriver::login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            trace!("cosign login -u {username} -p [MASKED] {registry}");
            let output = Command::new("cosign")
                .arg("login")
                .arg("-u")
                .arg(username)
                .arg("-p")
                .arg(password)
                .arg(registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker: {err_out}");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) enum VerifyType {
    File(String),
    Keyless { issuer: String, identity: String },
}

impl CosignDriver {
    fn sign(image_digest: &str, key_arg: Option<String>) -> Result<()> {
        let mut command = Command::new("cosign");
        command
            .env(COSIGN_PASSWORD, "")
            .env(COSIGN_YES, "true")
            .arg("sign");

        if let Some(key_arg) = key_arg {
            command.arg(key_arg);
        }

        command.arg("--recursive").arg(image_digest);

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to sign {image_digest}");
        }

        Ok(())
    }

    fn verify(image_name_tag: &str, verify_type: VerifyType) -> Result<()> {
        let mut command = Command::new("cosign");
        command.arg("verify");

        match verify_type {
            VerifyType::File(path) => command.arg(format!("--key={path}")),
            VerifyType::Keyless { issuer, identity } => command
                .arg("--certificate-identity-regexp")
                .arg(identity)
                .arg("--certificate-oidc-issuer")
                .arg(issuer),
        };

        command.arg(image_name_tag);

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to verify {image_name_tag}");
        }

        Ok(())
    }
}
