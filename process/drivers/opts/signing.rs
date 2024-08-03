use std::{borrow::Cow, env, fs, path::Path};

use miette::{IntoDiagnostic, Result};
use typed_builder::TypedBuilder;
use zeroize::Zeroizing;

pub enum PrivateKey {
    Env(&'static str),
    Path(&'static Path),
}

impl PrivateKey {
    /// Gets's the contents of the `PrivateKey`.
    ///
    /// # Errors
    /// Will error if the file or the environment couldn't be read.
    pub fn contents(&self) -> Result<Zeroizing<Vec<u8>>> {
        Ok(Zeroizing::new(match *self {
            Self::Env(env) => env::var(env).into_diagnostic()?.as_bytes().to_vec(),
            Self::Path(path) => fs::read(path).into_diagnostic()?,
        }))
    }
}

impl std::fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            match *self {
                Self::Env(env) => format!("env://{env}"),
                Self::Path(path) => format!("{}", path.display()),
            }
            .as_str(),
        )
    }
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct SignOpts<'scope> {
    #[builder(setter(into))]
    pub image: Cow<'scope, str>,

    #[builder(default, setter(into, strip_option))]
    pub key: Option<Cow<'scope, str>>,
}

#[derive(Debug, Clone)]
pub enum VerifyType<'scope> {
    File(Cow<'scope, str>),
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
}
