use std::{borrow::Cow, env, sync::Mutex};

use anyhow::{anyhow, Result};
use blue_build_utils::constants::*;
use lazy_static::lazy_static;
use typed_builder::TypedBuilder;

/// Stored user creds.
///
/// This is a special handoff static ref that is consumed
/// by the `ENV_CREDENTIALS` static ref. This can be set
/// at the beginning of a command for future calls for
/// creds to source from.
static USER_CREDS: Mutex<UserCreds> = Mutex::new(UserCreds {
    username: None,
    password: None,
    registry: None,
});

/// The credentials for logging into image registries.
#[derive(Debug, Default, Clone, TypedBuilder)]
pub struct Credentials {
    pub registry: String,
    pub username: String,
    pub password: String,
}

struct UserCreds {
    pub username: Option<String>,
    pub password: Option<String>,
    pub registry: Option<String>,
}

lazy_static! {
    /// Stores the global env credentials.
    ///
    /// This on load will determine the credentials based off of
    /// `USER_CREDS` and env vars from CI systems. Once this is called
    /// the value is stored and cannot change.
    ///
    /// If you have user
    /// provided credentials, make sure you update `USER_CREDS`
    /// before trying to access this reference.
    static ref ENV_CREDENTIALS: Option<Credentials> = {
        let (username, password, registry) = {
            USER_CREDS.lock().map_or((None, None, None), |creds| (
                creds.username.as_ref().map(|s| s.to_string()),
                creds.password.as_ref().map(|s| s.to_string()),
                creds.registry.as_ref().map(|s| s.to_string()),
            ))
        };

        let registry = match (
            registry.as_ref(),
            env::var(CI_REGISTRY).ok(),
            env::var(GITHUB_ACTIONS).ok(),
        ) {
            (Some(registry), _, _) => registry.to_owned(),
            (None, Some(ci_registry), None) => ci_registry,
            (None, None, Some(_)) => "ghcr.io".to_string(),
            _ => return None,
        };

        let username = match (
            username.as_ref(),
            env::var(CI_REGISTRY_USER).ok(),
            env::var(GITHUB_ACTOR).ok(),
        ) {
            (Some(username), _, _) => username.to_owned(),
            (None, Some(ci_registry_user), None) => ci_registry_user,
            (None, None, Some(github_actor)) => github_actor,
            _ => return None,
        };

        let password = match (
            password.as_ref(),
            env::var(CI_REGISTRY_PASSWORD).ok(),
            env::var(GITHUB_TOKEN).ok(),
        ) {
            (Some(password), _, _) => password.to_owned(),
            (None, Some(ci_registry_password), None) => ci_registry_password,
            (None, None, Some(registry_token)) => registry_token,
            _ => return None,
        };

        Some(
            Credentials::builder()
                .registry(registry)
                .username(username)
                .password(password)
                .build(),
        )
    };
}

/// Set the users credentials for
/// the current set of actions.
///
/// Be sure to call this before trying to use
/// any strategy that requires credentials as
/// the environment credentials are lazy allocated.
pub fn set_user_creds<'a>(
    username: Option<Cow<'a, str>>,
    password: Option<Cow<'a, str>>,
    registry: Option<Cow<'a, str>>,
) -> Result<()> {
    let mut creds_lock = USER_CREDS
        .lock()
        .map_err(|e| anyhow!("Failed to set credentials: {e}"))?;
    creds_lock.username = username.map(|s| s.to_string());
    creds_lock.password = password.map(|s| s.to_string());
    creds_lock.registry = registry.map(|s| s.to_string());
    drop(creds_lock);
    Ok(())
}

/// Get the credentials for the current set of actions.
///
/// # Errors
/// Will error if there aren't any credentials available.
pub fn get_credentials() -> Result<&'static Credentials> {
    ENV_CREDENTIALS
        .as_ref()
        .ok_or_else(|| anyhow!("No credentials available"))
}
