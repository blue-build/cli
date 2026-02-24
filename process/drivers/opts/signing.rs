use std::{
    fs,
    path::{Path, PathBuf},
};

use blue_build_utils::{
    constants::{BB_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PRIVATE_KEY, COSIGN_PUB_PATH},
    get_env_var,
    platform::Platform,
    string,
};
use bon::Builder;
use miette::{IntoDiagnostic, Result, bail};
use oci_client::Reference;
use zeroize::{Zeroize, Zeroizing};

use crate::drivers::types::ImageMetadata;

#[derive(Debug)]
pub enum PrivateKey {
    Env(String),
    Path(PathBuf),
}

impl PrivateKey {
    /// Create a `PrivateKey` object that tracks where the public key is.
    ///
    /// Contents of the `PrivateKey` are lazy loaded when `PrivateKey::contents` is called.
    ///
    /// # Errors
    ///
    /// Will error if the private key location cannot be found.
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        Ok(
            match (
                path.join(COSIGN_PUB_PATH).exists(),
                get_env_var(BB_PRIVATE_KEY).ok(),
                get_env_var(COSIGN_PRIVATE_KEY).ok(),
                path.join(COSIGN_PRIV_PATH),
            ) {
                (true, Some(private_key), _, _) if !private_key.is_empty() => {
                    Self::Env(string!(BB_PRIVATE_KEY))
                }
                (true, _, Some(cosign_priv_key), _) if !cosign_priv_key.is_empty() => {
                    Self::Env(string!(COSIGN_PRIVATE_KEY))
                }
                (true, _, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                    Self::Path(cosign_priv_key_path)
                }
                _ => {
                    bail!(
                        help = format!(
                            "{}{}{}{}{}{}",
                            format_args!("Make sure you have a `{COSIGN_PUB_PATH}`\n"),
                            format_args!(
                                "in the root of your repo and have either {COSIGN_PRIVATE_KEY}\n"
                            ),
                            format_args!("set in your env variables or a `{COSIGN_PRIV_PATH}`\n"),
                            "file in the root of your repo.\n\n",
                            "See https://blue-build.org/how-to/cosign/ for more information.\n\n",
                            "If you don't want to sign your image, use the `--no-sign` flag.",
                        ),
                        "{}",
                        "Unable to find private/public key pair",
                    )
                }
            },
        )
    }
}

impl std::fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match *self {
                Self::Env(ref env) => format!("env://{env}"),
                Self::Path(ref path) => format!("{}", path.display()),
            }
            .as_str(),
        )
    }
}

pub trait PrivateKeyContents<T>
where
    T: Zeroize,
{
    /// Gets's the contents of the `PrivateKey`.
    ///
    /// # Errors
    /// Will error if the file or the environment couldn't be read.
    fn contents(&self) -> Result<Zeroizing<T>>;
}

impl PrivateKeyContents<Vec<u8>> for PrivateKey {
    fn contents(&self) -> Result<Zeroizing<Vec<u8>>> {
        let key: Zeroizing<String> = self.contents()?;
        Ok(Zeroizing::new(key.as_bytes().to_vec()))
    }
}

impl PrivateKeyContents<String> for PrivateKey {
    fn contents(&self) -> Result<Zeroizing<String>> {
        Ok(Zeroizing::new(match *self {
            Self::Env(ref env) => get_env_var(env)?,
            Self::Path(ref path) => fs::read_to_string(path).into_diagnostic()?,
        }))
    }
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct GenerateKeyPairOpts<'scope> {
    pub dir: Option<&'scope Path>,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct CheckKeyPairOpts<'scope> {
    pub dir: Option<&'scope Path>,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct SignOpts<'scope> {
    pub image: &'scope Reference,
    pub metadata: &'scope ImageMetadata,
    pub key: Option<&'scope PrivateKey>,
}

#[derive(Debug, Clone, Copy)]
pub enum VerifyType<'scope> {
    File(&'scope Path),
    Keyless {
        issuer: &'scope str,
        identity: &'scope str,
    },
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct VerifyOpts<'scope> {
    pub image: &'scope Reference,
    pub verify_type: VerifyType<'scope>,
}

#[derive(Debug, Clone, Copy, Builder)]
#[builder(derive(Debug, Clone))]
pub struct SignVerifyOpts<'scope> {
    pub image: &'scope Reference,
    pub dir: Option<&'scope Path>,

    /// Enable retry logic for pushing.
    #[builder(default)]
    pub retry_push: bool,

    /// Number of times to retry pushing.
    ///
    /// Defaults to 1.
    #[builder(default = 1)]
    pub retry_count: u8,
    pub platforms: &'scope [Platform],
}
