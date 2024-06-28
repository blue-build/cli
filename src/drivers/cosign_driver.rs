use std::{env, fs, path::Path, process::Command};

use anyhow::{bail, Context};
use blue_build_utils::constants::{COSIGN_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
use log::{debug, trace};

use super::SigningDriver;

#[derive(Debug)]
pub struct CosignDriver;

impl SigningDriver for CosignDriver {
    fn generate_key_pair(&self) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
        // let status =
        todo!()
    }

    fn check_signing_files(&self) -> anyhow::Result<()> {
        env::set_var("COSIGN_PASSWORD", "");
        env::set_var("COSIGN_YES", "true");

        match (
            env::var(COSIGN_PRIVATE_KEY).ok(),
            Path::new(COSIGN_PRIV_PATH),
        ) {
            (Some(cosign_priv_key), _)
                if !cosign_priv_key.is_empty() && Path::new(COSIGN_PUB_PATH).exists() =>
            {
                trace!("cosign public-key --key env://COSIGN_PRIVATE_KEY");
                let output = Command::new("cosign")
                    .arg("public-key")
                    .arg("--key=env://COSIGN_PRIVATE_KEY")
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
            (None, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                trace!("cosign public-key --key {COSIGN_PRIV_PATH}");
                let output = Command::new("cosign")
                    .arg("public-key")
                    .arg(format!("--key={COSIGN_PRIV_PATH}"))
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
            _ => {
                bail!("Unable to find private/public key pair.\n\nMake sure you have a `{COSIGN_PUB_PATH}` in the root of your repo and have either {COSIGN_PRIVATE_KEY} set in your env variables or a `{COSIGN_PRIV_PATH}` file in the root of your repo.\n\nSee https://blue-build.org/how-to/cosign/ for more information.\n\nIf you don't want to sign your image, use the `--no-sign` flag.");
            }
        }
    }
}
