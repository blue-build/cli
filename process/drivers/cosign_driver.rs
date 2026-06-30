use std::{fmt::Debug, fs, path::Path};

use blue_build_utils::{
    constants::{
        BB_COSIGN_SIGN_ARGS, BB_COSIGN_VERIFY_ARGS, COSIGN_PASSWORD, COSIGN_PUB_PATH, COSIGN_YES,
    },
    credentials::Credentials,
    semver::Version,
};
use colored::Colorize;
use comlexr::{cmd, pipe};
use log::{debug, trace};
use miette::{Context, IntoDiagnostic, Result, bail};
use semver::VersionReq;
use serde::Deserialize;

use crate::drivers::{
    DriverVersion,
    opts::{PrivateKey, VerifyType},
};

use super::{
    SigningDriver,
    opts::{CheckKeyPairOpts, GenerateKeyPairOpts, SignOpts, VerifyOpts},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionJson {
    git_version: Version,
}

#[derive(Debug)]
pub struct CosignDriver;

impl CosignDriver {
    fn is_v3() -> bool {
        Self::version().is_ok_and(|version| {
            VersionReq::parse(">=3, <4").is_ok_and(|req| req.matches(&version))
        })
    }
}

impl DriverVersion for CosignDriver {
    const VERSION_REQ: &'static str = ">=2";

    fn version() -> Result<Version> {
        trace!("CosignDriver::version()");

        let output = {
            let c = cmd!("cosign", "version", "--json");
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

        let version_json: VersionJson = serde_json::from_slice(&output.stdout).into_diagnostic()?;

        Ok(version_json.git_version)
    }
}

impl SigningDriver for CosignDriver {
    fn generate_key_pair(opts: GenerateKeyPairOpts) -> Result<()> {
        let path = opts.dir.unwrap_or_else(|| Path::new("."));

        let status = {
            let c = cmd!(
                cd path;
                env {
                    COSIGN_PASSWORD: "",
                    COSIGN_YES: "true",
                };
                "cosign",
                "generate-key-pair",
            );
            trace!("{c:?}");
            c
        }
        .status()
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to generate cosign key-pair!");
        }

        Ok(())
    }

    fn check_signing_files(opts: CheckKeyPairOpts) -> Result<()> {
        let path = opts.dir.unwrap_or_else(|| Path::new("."));
        let priv_key = PrivateKey::new(path)?;

        let output = {
            let c = cmd!(
                env {
                    COSIGN_PASSWORD: "",
                    COSIGN_YES: "true"
                };
                "cosign",
                "public-key",
                format!("--key={priv_key}"),
            );
            trace!("{c:?}");
            c
        }
        .output()
        .into_diagnostic()?;

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

    fn signing_login(server: &str) -> Result<()> {
        trace!("CosignDriver::signing_login()");

        if let Some(Credentials::Basic { username, password }) = Credentials::get(server) {
            let output = pipe!(
                stdin = password.value();
                {
                    let c = cmd!(
                        "cosign",
                        "login",
                        "-u",
                        &username,
                        "--password-stdin",
                        server,
                    );
                    trace!("{c:?}");
                    c
                }
            )
            .output()
            .into_diagnostic()?;

            if !output.status.success() {
                let err_out = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to login for cosign:\n{}", err_out.trim());
            }
            debug!("Logged into {server}");
        }
        Ok(())
    }

    fn sign(
        SignOpts {
            image,
            metadata,
            key,
        }: SignOpts,
    ) -> Result<()> {
        let image = image.clone_with_digest(metadata.digest().into());
        let status = {
            let mut c = cmd!(
                env {
                    COSIGN_PASSWORD: "",
                    COSIGN_YES: "true",
                };
                "cosign",
                "sign",
                if let Some(key) = key => format!("--key={key}"),
                if Self::is_v3() => [
                    "--new-bundle-format=false",
                    "--use-signing-config=false",
                ],
                "--recursive",
                image.to_string(),
            );
            // Append operator-supplied flags to `cosign sign`, e.g.
            // BB_COSIGN_SIGN_ARGS="--tlog-upload=false". Unset => no change.
            if let Ok(extra) = std::env::var(BB_COSIGN_SIGN_ARGS) {
                c.args(split_extra_args(&extra));
            }
            trace!("{c:?}");
            c
        }
        .status()
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to sign {}", image.to_string().bold().red());
        }

        Ok(())
    }

    fn verify(opts: VerifyOpts) -> Result<()> {
        let status = {
            let mut c = cmd!(
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
            // Append operator-supplied flags to `cosign verify`, e.g.
            // BB_COSIGN_VERIFY_ARGS="--insecure-ignore-tlog=true". Unset => none.
            if let Ok(extra) = std::env::var(BB_COSIGN_VERIFY_ARGS) {
                c.args(split_extra_args(&extra));
            }
            trace!("{c:?}");
            c
        }
        .status()
        .into_diagnostic()?;

        if !status.success() {
            bail!("Failed to verify {}", opts.image.to_string().bold().red());
        }

        Ok(())
    }
}

/// Split an operator-supplied list of extra cosign flags (the value of
/// [`BB_COSIGN_SIGN_ARGS`]/[`BB_COSIGN_VERIFY_ARGS`]) into individual
/// arguments.
///
/// Tokens are separated by any run of ASCII whitespace, so a blank or
/// whitespace-only value yields no arguments. This intentionally does not
/// support shell quoting; each flag must be a single whitespace-free token
/// (e.g. `--tlog-upload=false`).
fn split_extra_args(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(ToString::to_string).collect()
}

#[cfg(test)]
mod test {
    use std::{fs, path::Path};

    use blue_build_utils::{
        constants::{COSIGN_PRIV_PATH, COSIGN_PUB_PATH},
        tempdir,
    };

    use crate::drivers::{
        SigningDriver,
        opts::{CheckKeyPairOpts, GenerateKeyPairOpts},
    };

    use super::{CosignDriver, split_extra_args};

    #[test]
    fn split_extra_args_parses_tokens() {
        // Unset/blank values contribute no arguments.
        assert!(split_extra_args("").is_empty());
        assert!(split_extra_args("   \t \n").is_empty());

        // A single flag.
        assert_eq!(
            split_extra_args("--tlog-upload=false"),
            ["--tlog-upload=false"]
        );

        // Multiple flags separated by arbitrary whitespace.
        assert_eq!(
            split_extra_args("  --insecure-ignore-tlog=true   --foo=bar\t--baz "),
            ["--insecure-ignore-tlog=true", "--foo=bar", "--baz"]
        );
    }

    #[test]
    fn generate_key_pair() {
        let tempdir = tempdir().unwrap();

        CosignDriver::generate_key_pair(GenerateKeyPairOpts::builder().dir(tempdir.path()).build())
            .unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        CosignDriver::check_signing_files(CheckKeyPairOpts::builder().dir(tempdir.path()).build())
            .unwrap();
    }

    #[test]
    fn check_key_pairs() {
        let path = Path::new("../test-files/keys");

        CosignDriver::check_signing_files(CheckKeyPairOpts::builder().dir(path).build()).unwrap();
    }

    #[test]
    fn compatibility() {
        use crate::drivers::sigstore_driver::SigstoreDriver;

        let tempdir = tempdir().unwrap();

        CosignDriver::generate_key_pair(GenerateKeyPairOpts::builder().dir(tempdir.path()).build())
            .unwrap();

        eprintln!(
            "Private key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PRIV_PATH)).unwrap()
        );
        eprintln!(
            "Public key:\n{}",
            fs::read_to_string(tempdir.path().join(COSIGN_PUB_PATH)).unwrap()
        );

        SigstoreDriver::check_signing_files(
            CheckKeyPairOpts::builder().dir(tempdir.path()).build(),
        )
        .unwrap();
    }
}
