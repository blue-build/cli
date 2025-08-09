use std::{
    fs,
    path::{Path, PathBuf},
};

use blue_build_utils::get_env_var;
use bon::Builder;
use miette::{IntoDiagnostic, Result};
use oci_distribution::Reference;
use zeroize::{Zeroize, Zeroizing};

use crate::drivers::types::Platform;

#[derive(Debug)]
pub enum PrivateKey {
    Env(String),
    Path(PathBuf),
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
pub struct GenerateKeyPairOpts<'scope> {
    pub dir: Option<&'scope Path>,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct CheckKeyPairOpts<'scope> {
    pub dir: Option<&'scope Path>,
}

#[derive(Debug, Clone, Copy, Builder)]
pub struct SignOpts<'scope> {
    pub image: &'scope Reference,
    pub key: Option<&'scope PrivateKey>,
    pub dir: Option<&'scope Path>,
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
pub struct VerifyOpts<'scope> {
    pub image: &'scope Reference,
    pub verify_type: VerifyType<'scope>,
}

#[derive(Debug, Clone, Copy, Builder)]
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

    pub platform: Option<Platform>,
}
