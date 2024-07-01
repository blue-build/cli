use std::{env, fmt::Debug, fs, path::Path, process::Command};

use anyhow::{bail, Context, Result};
use blue_build_utils::constants::{
    CI_DEFAULT_BRANCH, CI_PROJECT_URL, CI_SERVER_HOST, CI_SERVER_PROTOCOL, COSIGN_PRIVATE_KEY,
    COSIGN_PRIV_PATH, COSIGN_PUB_PATH, GITHUB_TOKEN, GITHUB_TOKEN_ISSUER_URL, GITHUB_WORKFLOW_REF,
    SIGSTORE_ID_TOKEN,
};
use log::{debug, info, trace, warn};

use crate::drivers::{opts::GetMetadataOpts, Driver, InspectDriver};

use super::SigningDriver;

#[derive(Debug)]
pub struct CosignDriver;

impl SigningDriver for CosignDriver {
    fn generate_key_pair() -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
        // let status =
        todo!()
    }

    fn check_signing_files() -> anyhow::Result<()> {
        env::set_var("COSIGN_PASSWORD", "");
        env::set_var("COSIGN_YES", "true");

        match (
            env::var(COSIGN_PRIVATE_KEY).ok(),
            Path::new(COSIGN_PRIV_PATH),
        ) {
            (Some(cosign_priv_key), _)
                if !cosign_priv_key.is_empty() && Path::new(COSIGN_PUB_PATH).exists() =>
            {
                Self::check_priv("env://COSIGN_PRIVATE_KEY")
            }
            (_, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                Self::check_priv(cosign_priv_key_path.display().to_string())
            }
            _ => {
                bail!(concat!(
                    "Unable to find private/public key pair.\n\n",
                    "Make sure you have a `{COSIGN_PUB_PATH}` ",
                    "in the root of your repo and have either {COSIGN_PRIVATE_KEY} ",
                    "set in your env variables or a `{COSIGN_PRIV_PATH}` ",
                    "file in the root of your repo.\n\n",
                    "See https://blue-build.org/how-to/cosign/ for more information.\n\n",
                    "If you don't want to sign your image, use the `--no-sign` flag."
                ));
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn sign_images<S, T>(image_name: S, tag: Option<T>) -> Result<()>
    where
        S: AsRef<str>,
        T: AsRef<str> + Debug,
    {
        let image_name = image_name.as_ref();
        trace!("BuildCommand::sign_images({image_name}, {tag:?})");

        env::set_var("COSIGN_PASSWORD", "");
        env::set_var("COSIGN_YES", "true");

        let inspect_opts = GetMetadataOpts::builder().image(image_name);

        let inspect_opts = if let Some(tag) = tag.as_ref() {
            inspect_opts.tag(tag.as_ref()).build()
        } else {
            inspect_opts.build()
        };

        let image_digest = Driver::get_metadata(&inspect_opts)?.digest;
        let image_name_digest = format!("{image_name}@{image_digest}");
        let image_name_tag = tag.map_or_else(
            || image_name.to_owned(),
            |t| format!("{image_name}:{}", t.as_ref()),
        );

        match (
            // GitLab specific vars
            env::var(CI_DEFAULT_BRANCH),
            env::var(CI_PROJECT_URL),
            env::var(CI_SERVER_PROTOCOL),
            env::var(CI_SERVER_HOST),
            env::var(SIGSTORE_ID_TOKEN),
            // GitHub specific vars
            env::var(GITHUB_TOKEN),
            env::var(GITHUB_WORKFLOW_REF),
            // Cosign public/private key pair
            env::var(COSIGN_PRIVATE_KEY),
            Path::new(COSIGN_PRIV_PATH),
        ) {
            // Cosign public/private key pair
            (_, _, _, _, _, _, _, Ok(cosign_private_key), _)
                if !cosign_private_key.is_empty() && Path::new(COSIGN_PUB_PATH).exists() =>
            {
                Self::sign_priv_public_pair_env(&image_name_digest, &image_name_tag)?;
            }
            (_, _, _, _, _, _, _, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                Self::sign_priv_public_pair_file(&image_name_digest, &image_name_tag)?;
            }
            // Gitlab keyless
            (
                Ok(ci_default_branch),
                Ok(ci_project_url),
                Ok(ci_server_protocol),
                Ok(ci_server_host),
                Ok(_),
                _,
                _,
                _,
                _,
            ) => {
                trace!("CI_PROJECT_URL={ci_project_url}, CI_DEFAULT_BRANCH={ci_default_branch}, CI_SERVER_PROTOCOL={ci_server_protocol}, CI_SERVER_HOST={ci_server_host}");

                info!("Signing image: {image_name_digest}");

                trace!("cosign sign {image_name_digest}");

                if Command::new("cosign")
                    .arg("sign")
                    .arg("--recursive")
                    .arg(&image_name_digest)
                    .status()?
                    .success()
                {
                    info!("Successfully signed image!");
                } else {
                    bail!("Failed to sign image: {image_name_digest}");
                }

                let cert_ident =
                    format!("{ci_project_url}//.gitlab-ci.yml@refs/heads/{ci_default_branch}");

                let cert_oidc = format!("{ci_server_protocol}://{ci_server_host}");

                trace!("cosign verify --certificate-identity {cert_ident} --certificate-oidc-issuer {cert_oidc} {image_name_tag}");

                if !Command::new("cosign")
                    .arg("verify")
                    .arg("--certificate-identity")
                    .arg(&cert_ident)
                    .arg("--certificate-oidc-issuer")
                    .arg(&cert_oidc)
                    .arg(&image_name_tag)
                    .status()?
                    .success()
                {
                    bail!("Failed to verify image!");
                }
            }
            // GitHub keyless
            (_, _, _, _, _, Ok(_), Ok(github_worflow_ref), _, _) => {
                trace!("GITHUB_WORKFLOW_REF={github_worflow_ref}");

                info!("Signing image {image_name_digest}");

                trace!("cosign sign {image_name_digest}");
                if Command::new("cosign")
                    .arg("sign")
                    .arg("--recursive")
                    .arg(&image_name_digest)
                    .status()?
                    .success()
                {
                    info!("Successfully signed image!");
                } else {
                    bail!("Failed to sign image: {image_name_digest}");
                }

                trace!("cosign verify --certificate-identity-regexp {github_worflow_ref} --certificate-oidc-issuer {GITHUB_TOKEN_ISSUER_URL} {image_name_tag}");
                if !Command::new("cosign")
                    .arg("verify")
                    .arg("--certificate-identity-regexp")
                    .arg(&github_worflow_ref)
                    .arg("--certificate-oidc-issuer")
                    .arg(GITHUB_TOKEN_ISSUER_URL)
                    .arg(&image_name_tag)
                    .status()?
                    .success()
                {
                    bail!("Failed to verify image!");
                }
            }
            _ => warn!("Not running in CI with cosign variables, not signing"),
        }

        Ok(())
    }
}

impl CosignDriver {
    fn check_priv<S>(priv_key: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        let priv_key = priv_key.as_ref();

        trace!("cosign public-key --key {priv_key}");
        let output = Command::new("cosign")
            .arg("public-key")
            .arg(format!("--key={priv_key}"))
            .output()?;

        if !output.status.success() {
            bail!(
                "Failed to run cosign public-key: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let calculated_pub_key = String::from_utf8(output.stdout)?;
        let found_pub_key = fs::read_to_string(COSIGN_PUB_PATH)
            .with_context(|| format!("Failed to read {COSIGN_PUB_PATH}"))?;
        trace!("calculated_pub_key={calculated_pub_key},found_pub_key={found_pub_key}");

        if calculated_pub_key.trim() == found_pub_key.trim() {
            debug!("Cosign files match, continuing build");
            Ok(())
        } else {
            bail!("Public key '{COSIGN_PUB_PATH}' does not match private key")
        }
    }

    #[allow(clippy::too_many_lines)]

    fn sign_priv_public_pair_env(image_digest: &str, image_name_tag: &str) -> Result<()> {
        info!("Signing image: {image_digest}");

        trace!("cosign sign --key=env://{COSIGN_PRIVATE_KEY} {image_digest}");

        if Command::new("cosign")
            .arg("sign")
            .arg("--key=env://COSIGN_PRIVATE_KEY")
            .arg("--recursive")
            .arg(image_digest)
            .status()?
            .success()
        {
            info!("Successfully signed image!");
        } else {
            bail!("Failed to sign image: {image_digest}");
        }

        trace!("cosign verify --key {COSIGN_PUB_PATH} {image_name_tag}");

        if !Command::new("cosign")
            .arg("verify")
            .arg(format!("--key={COSIGN_PUB_PATH}"))
            .arg(image_name_tag)
            .status()?
            .success()
        {
            bail!("Failed to verify image!");
        }

        Ok(())
    }

    fn sign_priv_public_pair_file(image_digest: &str, image_name_tag: &str) -> Result<()> {
        info!("Signing image: {image_digest}");

        trace!("cosign sign --key={COSIGN_PRIV_PATH} {image_digest}");

        if Command::new("cosign")
            .arg("sign")
            .arg(format!("--key={COSIGN_PRIV_PATH}"))
            .arg("--recursive")
            .arg(image_digest)
            .status()?
            .success()
        {
            info!("Successfully signed image!");
        } else {
            bail!("Failed to sign image: {image_digest}");
        }

        trace!("cosign verify --key {COSIGN_PUB_PATH} {image_name_tag}");

        if !Command::new("cosign")
            .arg("verify")
            .arg(format!("--key={COSIGN_PUB_PATH}"))
            .arg(image_name_tag)
            .status()?
            .success()
        {
            bail!("Failed to verify image!");
        }

        Ok(())
    }
}
