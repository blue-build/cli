use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, Result};
use typed_builder::TypedBuilder;
use zeroize::{Zeroize, Zeroizing};

pub enum PrivateKey {
    Env(String),
    Path(PathBuf),
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

#[derive(Debug, Clone, TypedBuilder)]
pub struct GenerateKeyPairOpts<'scope> {
    #[builder(setter(into, strip_option))]
    pub dir: Option<Cow<'scope, Path>>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct CheckKeyPairOpts<'scope> {
    #[builder(setter(into, strip_option))]
    pub dir: Option<Cow<'scope, Path>>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct SignOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,

    #[builder(default, setter(into, strip_option))]
    pub key: Option<Cow<'scope, str>>,

    #[builder(default, setter(into, strip_option))]
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

#[derive(Debug, Clone, TypedBuilder)]
pub struct VerifyOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,
    pub verify_type: VerifyType<'scope>,
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct SignVerifyOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,

    #[builder(default, setter(into, strip_option))]
    pub tag: Option<Cow<'scope, str>>,

    #[builder(default, setter(into, strip_option))]
    pub dir: Option<Cow<'scope, Path>>,
}
