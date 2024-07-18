use std::{env, fmt::Debug, fs, path::Path, process::Command};

use blue_build_utils::constants::{
    COSIGN_PASSWORD, COSIGN_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PUB_PATH, COSIGN_YES,
};
use log::{debug, trace, warn};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::{
    credentials::Credentials,
    drivers::{opts::GetMetadataOpts, types::CiDriverType, CiDriver, Driver, InspectDriver},
};

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
        match (
            Path::new(COSIGN_PUB_PATH).exists(),
            env::var(COSIGN_PRIVATE_KEY).ok(),
            Path::new(COSIGN_PRIV_PATH),
        ) {
            (true, Some(cosign_priv_key), _) if !cosign_priv_key.is_empty() => {
                Self::check_priv("env://COSIGN_PRIVATE_KEY")
            }
            (true, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                Self::check_priv(cosign_priv_key_path.display().to_string())
            }
            (true, _, _) => {
                bail!(
                    "{}{}{}{}{}{}{}",
                    "Unable to find private/public key pair.\n\n",
                    format_args!("Make sure you have a `{COSIGN_PUB_PATH}` "),
                    format_args!("in the root of your repo and have either {COSIGN_PRIVATE_KEY} "),
                    format_args!("set in your env variables or a `{COSIGN_PRIV_PATH}` "),
                    "file in the root of your repo.\n\n",
                    "See https://blue-build.org/how-to/cosign/ for more information.\n\n",
                    "If you don't want to sign your image, use the `--no-sign` flag."
                )
            }
            _ => Ok(()),
        }
    }

    fn sign_images<S, T>(image_name: S, tag: Option<T>) -> Result<()>
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        let image_name = image_name.as_ref();
        let tag = tag.as_ref().map(AsRef::as_ref);
        trace!("BuildCommand::sign_images({image_name}, {tag:?})");

        let inspect_opts = GetMetadataOpts::builder().image(image_name);

        let inspect_opts = if let Some(tag) = tag {
            inspect_opts.tag(tag).build()
        } else {
            inspect_opts.build()
        };

        let image_digest = Driver::get_metadata(&inspect_opts)?.digest;
        let image_name_tag =
            tag.map_or_else(|| image_name.to_owned(), |t| format!("{image_name}:{t}"));
        let image_digest = format!("{image_name}@{image_digest}");

        match (
            Driver::get_ci_driver(),
            // Cosign public/private key pair
            env::var(COSIGN_PRIVATE_KEY),
            Path::new(COSIGN_PRIV_PATH),
        ) {
            // Cosign public/private key pair
            (_, Ok(cosign_private_key), _)
                if !cosign_private_key.is_empty() && Path::new(COSIGN_PUB_PATH).exists() =>
            {
                Self::sign(
                    image_digest,
                    Some(&format!("--key=env://{COSIGN_PRIVATE_KEY}")),
                )?;
                Self::verify(image_name_tag, VerifyType::File(COSIGN_PUB_PATH.into()))?;
            }
            (_, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                Self::sign(
                    image_digest,
                    Some(&format!("--key={}", cosign_priv_key_path.display())),
                )?;
                Self::verify(image_name_tag, VerifyType::File(COSIGN_PUB_PATH.into()))?;
            }
            // Gitlab keyless
            (CiDriverType::Github | CiDriverType::Gitlab, _, _) => {
                Self::sign(image_digest, None)?;
                Self::verify(
                    image_name_tag,
                    VerifyType::Keyless {
                        issuer: Driver::oidc_provider()?,
                        identity: Driver::keyless_cert_identity()?,
                    },
                )?;
            }
            _ => warn!("Not running in CI with cosign variables, not signing"),
        }

        Ok(())
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
    fn check_priv<S>(priv_key: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        let priv_key = priv_key.as_ref();

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
    }

    fn sign<S>(image_digest: S, key_arg: Option<&str>) -> Result<()>
    where
        S: AsRef<str>,
    {
        let image_digest = image_digest.as_ref();

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

    fn verify<S>(image_name_tag: S, verify_type: VerifyType) -> Result<()>
    where
        S: AsRef<str>,
    {
        let image_name_tag = image_name_tag.as_ref();
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
