use std::{fmt::Debug, fs, io::Write, path::Path, process::Stdio};

use blue_build_utils::{
    constants::{COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES},
    credentials::Credentials,
};
use colored::Colorize;
use comlexr::cmd;
use log::{debug, trace};
use miette::{bail, miette, Context, IntoDiagnostic, Result};

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

        let mut command = cmd!(
            "cosign",
            "generate-key-pair",
            // COSIGN_PASSWORD => "",
            // COSIGN_YES => "true",
        );
        command
            .current_dir(path)
            .env(COSIGN_PASSWORD, "")
            .env(COSIGN_YES, "true");

        let status = command.status().into_diagnostic()?;

        if !status.success() {
            bail!("Failed to generate cosign key-pair!");
        }

        Ok(())
    }

    fn check_signing_files(opts: &CheckKeyPairOpts) -> Result<()> {
        let path = opts.dir.as_ref().map_or_else(|| Path::new("."), |dir| dir);
        let priv_key = get_private_key(path)?;

        let mut command = cmd!(
            "cosign",
            "public-key",
            format!("--key={priv_key}"),
            // COSIGN_PASSWORD => "",
            // COSIGN_YES => "true",
        );
        command.env(COSIGN_PASSWORD, "").env(COSIGN_YES, "true");

        trace!("{command:?}");
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
                registry,
            );
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            trace!("{command:?}");
            let mut child = command.spawn().into_diagnostic()?;

            write!(
                child
                    .stdin
                    .as_mut()
                    .ok_or_else(|| miette!("Unable to open pipe to stdin"))?,
                "{password}"
            )
            .into_diagnostic()?;

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
        if opts.image.digest().is_none() {
            bail!(
                "Image ref {} is not a digest ref",
                opts.image.to_string().bold().red(),
            );
        }

        let mut command = cmd!(
            "cosign",
            "sign",
            if let Some(ref key) = opts.key => format!("--key={key}"),
            "--recursive",
            opts.image.to_string(),
            // COSIGN_PASSWORD => "",
            // COSIGN_YES => "true",
        );
        command.env(COSIGN_PASSWORD, "").env(COSIGN_YES, "true");

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to sign {}", opts.image.to_string().bold().red());
        }

        Ok(())
    }

    fn verify(opts: &VerifyOpts) -> Result<()> {
        let mut command = cmd!(
            "cosign",
            "verify",
            match &opts.verify_type {
                VerifyType::File(path) => format!("--key={}", path.display()),
                VerifyType::Keyless { issuer, identity } => [
                    "--certificate-identity-regexp",
                    &**identity,
                    "--certificate-oidc-issuer",
                    &**issuer,
                ],
            },
            opts.image.to_string(),
        );

        trace!("{command:?}");
        if !command.status().into_diagnostic()?.success() {
            bail!("Failed to verify {}", opts.image.to_string().bold().red());
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use blue_build_utils::constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH};
    use tempfile::TempDir;

    use crate::drivers::{
        opts::{CheckKeyPairOpts, GenerateKeyPairOpts},
        SigningDriver,
    };

    use super::CosignDriver;

    #[test]
    fn generate_key_pair() {
        let tempdir = TempDir::new().unwrap();

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
    #[cfg(feature = "sigstore")]
    fn compatibility() {
        use crate::drivers::sigstore_driver::SigstoreDriver;

        let tempdir = TempDir::new().unwrap();

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
