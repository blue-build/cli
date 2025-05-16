use std::path::Path;

use blue_build_utils::{
    constants::{BB_PRIVATE_KEY, COSIGN_PRIV_PATH, COSIGN_PRIVATE_KEY, COSIGN_PUB_PATH},
    get_env_var, string,
};
use miette::{Result, bail};

use super::opts::PrivateKey;

pub(super) fn get_private_key<P>(path: P) -> Result<PrivateKey>
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
                PrivateKey::Env(string!(BB_PRIVATE_KEY))
            }
            (true, _, Some(cosign_priv_key), _) if !cosign_priv_key.is_empty() => {
                PrivateKey::Env(string!(COSIGN_PRIVATE_KEY))
            }
            (true, _, _, cosign_priv_key_path) if cosign_priv_key_path.exists() => {
                PrivateKey::Path(cosign_priv_key_path)
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
