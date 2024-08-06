use std::{fmt::Debug, fs, io::Write, path::Path, process::Stdio};

use blue_build_utils::{
    cmd, cmd_env,
    constants::{COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES},
    credentials::Credentials,
};
use log::{debug, trace};
use miette::{bail, Context, IntoDiagnostic, Result};

use crate::drivers::opts::VerifyType;

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

        let mut command = cmd!("cosign", "generate-key-pair");
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
            let found_pub_key = fs::read_to_string(path.join(COSIGN_PUB_PATH))
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
            let mut command = cmd!(
                "cosign",
                "login",
                "-u",
                username,
                "--password-stdin",
                registry
            );
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            trace!("{command:?}");
            let mut child = command.spawn().into_diagnostic()?;

            write!(child.stdin.as_mut().unwrap(), "{password}").into_diagnostic()?;

            let output = child.wait_with_output().into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for cosign:\n{}", err_out.trim());
            }
            debug!("Logged into {registry}");
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

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
    use tempdir::TempDir;

    use crate::drivers::{
        opts::{CheckKeyPairOpts, GenerateKeyPairOpts},
        sigstore_driver::SigstoreDriver,
        SigningDriver,
    };

    use super::CosignDriver;

    #[test]
    fn generate_key_pair() {
        let tempdir = TempDir::new("keypair").unwrap();

        let gen_opts = GenerateKeyPairOpts::builder().dir(tempdir.path()).build();

        CosignDriver::generate_key_pair(&gen_opts).unwrap();

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

    #[test]
    fn check_key_pairs() {
        let path = Path::new("../test-files/keys");

        let opts = CheckKeyPairOpts::builder().dir(path).build();

        CosignDriver::check_signing_files(&opts).unwrap();
    }

    #[test]
    fn compatibility() {
        let tempdir = TempDir::new("keypair").unwrap();

        let gen_opts = GenerateKeyPairOpts::builder().dir(tempdir.path()).build();

        CosignDriver::generate_key_pair(&gen_opts).unwrap();

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
}
