use std::{env, fs, path::Path};

use blue_build_utils::constants::{
    BB_PRIVATE_KEY, COSIGN_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PUB_PATH,
};
use miette::{bail, IntoDiagnostic, Result};
use zeroize::Zeroizing;

pub(super) enum PrivateKey {
    Env(&'static str),
    Path(&'static Path),
}

impl PrivateKey {
    pub fn contents(&self) -> Result<Zeroizing<String>> {
        Ok(Zeroizing::new(match *self {
            Self::Env(env) => env::var(env).into_diagnostic()?,
            Self::Path(path) => fs::read_to_string(path).into_diagnostic()?,
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

pub(super) fn get_private_key<T>(check_fn: impl FnOnce(PrivateKey) -> Result<T>) -> Result<T> {
    match (
        Path::new(COSIGN_PUB_PATH).exists(),
        env::var(BB_PRIVATE_KEY).ok(),
        env::var(COSIGN_PRIVATE_KEY).ok(),
        Path::new(COSIGN_PRIV_PATH),
    ) {
        (true, Some(private_key), _, _) if !private_key.is_empty() => {
            check_fn(PrivateKey::Env(BB_PRIVATE_KEY))
        }
        (true, _, Some(cosign_priv_key), _) if !cosign_priv_key.is_empty() => {
            check_fn(PrivateKey::Env(COSIGN_PRIVATE_KEY))
        }
        (true, _, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
            check_fn(PrivateKey::Path(cosign_priv_key_path))
        }
        _ => {
            bail!(
                "{}{}{}{}{}{}{}",
                "Unable to find private/public key pair.\n\n",
                format_args!("Make sure you have a `{COSIGN_PUB_PATH}` "),
                format_args!("in the root of your repo and have either {COSIGN_PRIVATE_KEY} "),
                format_args!("set in your env variables or a `{COSIGN_PRIV_PATH}` "),
                "file in the root of your repo.\n\n",
                "See https://blue-build.org/how-to/cosign/ for more information.\n\n",
                "If you don't want to sign your image, use the `--no-sign` flag."
            )
        }
    }
}
