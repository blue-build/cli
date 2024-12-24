use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
};

use bon::Builder;
use miette::{IntoDiagnostic, Result};
use oci_distribution::Reference;
use zeroize::{Zeroize, Zeroizing};

use crate::drivers::types::Platform;

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
            Self::Env(ref env) => env::var(env).into_diagnostic()?,
            Self::Path(ref path) => fs::read_to_string(path).into_diagnostic()?,
        }))
    }
}

#[derive(Debug, Clone, Builder)]
pub struct GenerateKeyPairOpts<'scope> {
    #[builder(into)]
    pub dir: Option<Cow<'scope, Path>>,
}

#[derive(Debug, Clone, Builder)]
pub struct CheckKeyPairOpts<'scope> {
    #[builder(into)]
    pub dir: Option<Cow<'scope, Path>>,
}

#[derive(Debug, Clone, Builder)]
pub struct SignOpts<'scope> {
    #[builder(into)]
    pub image: &'scope Reference,

    #[builder(into)]
    pub key: Option<Cow<'scope, str>>,

    #[builder(into)]
    pub dir: Option<Cow<'scope, Path>>,
}

#[derive(Debug, Clone)]
pub enum VerifyType<'scope> {
    File(Cow<'scope, Path>),
    Keyless {
        issuer: Cow<'scope, str>,
        identity: Cow<'scope, str>,
    },
}

#[derive(Debug, Clone, Builder)]
pub struct VerifyOpts<'scope> {
    #[builder(into)]
    pub image: &'scope Reference,
    pub verify_type: VerifyType<'scope>,
}

#[derive(Debug, Clone, Builder)]
pub struct SignVerifyOpts<'scope> {
    #[builder(into)]
    pub image: &'scope Reference,

    #[builder(into)]
    pub dir: Option<Cow<'scope, Path>>,

    /// Enable retry logic for pushing.
    #[builder(default)]
    pub retry_push: bool,

    /// Number of times to retry pushing.
    ///
    /// Defaults to 1.
    #[builder(default = 1)]
    pub retry_count: u8,

    #[builder(default)]
    pub platform: Platform,
}
