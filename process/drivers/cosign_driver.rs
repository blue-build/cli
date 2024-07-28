use std::{fmt::Debug, fs};

use blue_build_utils::{
    cmd, cmd_env,
    constants::{COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES},
};
use log::{debug, trace};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::{credentials::Credentials, drivers::VerifyType};

use super::{functions::get_private_key, SigningDriver};

#[derive(Debug)]
pub struct CosignDriver;

impl SigningDriver for CosignDriver {
    fn generate_key_pair() -> Result<()> {
        let mut command = cmd!("cosign", "genereate-key-pair");
        cmd_env! {
            command,
            COSIGN_PASSWORD => "",
            COSIGN_YES => "true",
        };

        let status = command.status().into_diagnostic()?;

        if !status.success() {
            bail!("Failed to generate cosign key-pair!");
        }

        Ok(())
    }

    fn check_signing_files() -> Result<()> {
        get_private_key(|priv_key| {
            trace!("cosign public-key --key {priv_key}");
            let mut command = cmd!("cosign", "public-key", format!("--key={priv_key}"));
            cmd_env! {
                command,
                COSIGN_PASSWORD => "",
                COSIGN_YES => "true",
            };

            let output = command.output().into_diagnostic()?;

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

    fn signing_login() -> Result<()> {
        trace!("CosignDriver::signing_login()");

        if let Some(Credentials {
            registry,
            username,
            password,
        }) = Credentials::get()
        {
            trace!("cosign login -u {username} -p [MASKED] {registry}");
            let output = cmd!("cosign", "login", "-u", username, "-p", password, registry)
                .output()
                .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for docker: {err_out}");
            }
        }
        Ok(())
    }

    fn sign(image_digest: &str, key_arg: Option<String>) -> Result<()> {
        let mut command = cmd!("cosign", "sign");
        cmd_env! {
            command,
            COSIGN_PASSWORD => "",
            COSIGN_YES => "true",
        };

        if let Some(key_arg) = key_arg {
            cmd!(command, key_arg);
        }

        cmd!(command, "--recursive", image_digest);

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to sign {image_digest}");
        }

        Ok(())
    }

    fn verify(image_name_tag: &str, verify_type: VerifyType) -> Result<()> {
        let mut command = cmd!("cosign", "verify");

        match verify_type {
            VerifyType::File(path) => cmd!(command, format!("--key={path}")),
            VerifyType::Keyless { issuer, identity } => cmd!(
                command,
                "--certificate-identity-regexp",
                identity,
                "--certificate-oidc-issuer",
                issuer
            ),
        };

        cmd!(command, image_name_tag);

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to verify {image_name_tag}");
        }

        Ok(())
    }
}
