use std::{fmt::Debug, fs, path::Path};

use blue_build_utils::{
    cmd, cmd_env,
    constants::{COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES},
};
use log::{debug, trace};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::{credentials::Credentials, drivers::opts::VerifyType};

use super::{
    functions::get_private_key,
    opts::{CheckKeyPairOpts, GenerateKeyPairOpts, SignOpts, VerifyOpts},
    SigningDriver,
};

#[derive(Debug)]
pub struct CosignDriver;

impl SigningDriver for CosignDriver {
    fn generate_key_pair(opts: &GenerateKeyPairOpts) -> Result<()> {
        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);

        let mut command = cmd!("cosign", "genereate-key-pair");
        cmd_env! {
            command,
            COSIGN_PASSWORD => "",
            COSIGN_YES => "true",
        };
        command.current_dir(path);

        let status = command.status().into_diagnostic()?;

        if !status.success() {
            bail!("Failed to generate cosign key-pair!");
        }

        Ok(())
    }

    fn check_signing_files(opts: &CheckKeyPairOpts) -> Result<()> {
        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);
        get_private_key(path, |priv_key| {
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

    fn sign(opts: &SignOpts) -> Result<()> {
        let image_digest: &str = opts.image.as_ref();
        let mut command = cmd!("cosign", "sign");
        cmd_env! {
            command,
            COSIGN_PASSWORD => "",
            COSIGN_YES => "true",
        };

        if let Some(ref key) = opts.key {
            cmd!(command, format!("--key={key}"));
        }

        cmd!(command, "--recursive", image_digest);

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to sign {image_digest}");
        }

        Ok(())
    }

    fn verify(opts: &VerifyOpts) -> Result<()> {
        let image_name_tag: &str = opts.image.as_ref();
        let mut command = cmd!("cosign", "verify");

        match opts.verify_type {
            VerifyType::File(ref path) => cmd!(command, format!("--key={path}")),
            VerifyType::Keyless {
                ref issuer,
                ref identity,
            } => cmd!(
                command,
                "--certificate-identity-regexp",
                identity as &str,
                "--certificate-oidc-issuer",
                issuer as &str,
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
