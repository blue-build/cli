use std::{
    env,
    sync::{LazyLock, Mutex},
};

use bon::Builder;
use clap::Args;
use docker_credential::DockerCredential;
use log::trace;

use crate::{
    constants::{
        BB_PASSWORD, BB_REGISTRY, BB_USERNAME, CI_REGISTRY, CI_REGISTRY_PASSWORD, CI_REGISTRY_USER,
        GITHUB_ACTIONS, GITHUB_ACTOR, GITHUB_TOKEN,
    },
    string,
};

static INIT: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// Stored user creds.
///
/// This is a special handoff static ref that is consumed
/// by the `ENV_CREDENTIALS` static ref. This can be set
/// at the beginning of a command for future calls for
/// creds to source from.
static INIT_CREDS: Mutex<CredentialsArgs> = Mutex::new(CredentialsArgs {
    username: None,
    password: None,
    registry: None,
});

/// Stores the global env credentials.
///
/// This on load will determine the credentials based off of
/// `USER_CREDS` and env vars from CI systems. Once this is called
/// the value is stored and cannot change.
///
/// If you have user
/// provided credentials, make sure you update `USER_CREDS`
/// before trying to access this reference.
static ENV_CREDENTIALS: LazyLock<Option<Credentials>> = LazyLock::new(|| {
    let (username, password, registry) = {
        INIT_CREDS.lock().map_or((None, None, None), |mut creds| {
            (
                creds.username.take(),
                creds.password.take(),
                creds.registry.take(),
            )
        })
    };

    let registry = match (
        registry,
        env::var(CI_REGISTRY).ok(),
        env::var(GITHUB_ACTIONS).ok(),
    ) {
        (Some(registry), _, _) | (_, Some(registry), _) if !registry.is_empty() => registry,
        (_, _, Some(_)) => string!("ghcr.io"),
        _ => return None,
    };
    trace!("Registry: {registry:?}");

    let (username, password) = match (
        (username, password),
        docker_credential::get_credential(&registry).ok(),
        docker_credential::get_podman_credential(&registry).ok(),
        (
            env::var(CI_REGISTRY_USER).ok(),
            env::var(CI_REGISTRY_PASSWORD).ok(),
        ),
        (env::var(GITHUB_ACTOR).ok(), env::var(GITHUB_TOKEN).ok()),
    ) {
        ((Some(username), Some(password)), _, _, _, _)
        | (_, Some(DockerCredential::UsernamePassword(username, password)), _, _, _)
        | (_, _, Some(DockerCredential::UsernamePassword(username, password)), _, _)
        | (_, _, _, (Some(username), Some(password)), _)
        | (_, _, _, _, (Some(username), Some(password)))
            if !username.is_empty() && !password.is_empty() =>
        {
            (username, password)
        }
        _ => return None,
    };
    trace!("Username: {username}");

    Some(
        Credentials::builder()
            .registry(registry)
            .username(username)
            .password(password)
            .build(),
    )
});

/// The credentials for logging into image registries.
#[derive(Debug, Default, Clone, Builder)]
pub struct Credentials {
    pub registry: String,
    pub username: String,
    pub password: String,
}

impl Credentials {
    /// Set the users credentials for
    /// the current set of actions.
    ///
    /// Be sure to call this before trying to use
    /// any strategy that requires credentials as
    /// the environment credentials are lazy allocated.
    ///
    /// # Panics
    /// Will panic if it can't lock the mutex.
    pub fn init(args: CredentialsArgs) {
        trace!("Credentials::init()");
        let mut initialized = INIT.lock().expect("Must lock INIT");

        if !*initialized {
            let mut creds_lock = INIT_CREDS.lock().expect("Must lock USER_CREDS");
            *creds_lock = args;
            drop(creds_lock);
            let _ = ENV_CREDENTIALS.as_ref();

            *initialized = true;
        }
    }

    /// Get the credentials for the current set of actions.
    pub fn get() -> Option<&'static Self> {
        trace!("credentials::get()");
        ENV_CREDENTIALS.as_ref()
    }
}

#[derive(Debug, Default, Clone, Builder, Args)]
#[builder(on(String, into))]
pub struct CredentialsArgs {
    /// The registry's domain name.
    #[arg(long, env = BB_REGISTRY)]
    pub registry: Option<String>,

    /// The username to login to the
    /// container registry.
    #[arg(short = 'U', long, env = BB_USERNAME, hide_env_values = true)]
    pub username: Option<String>,

    /// The password to login to the
    /// container registry.
    #[arg(short = 'P', long, env = BB_PASSWORD, hide_env_values = true)]
    pub password: Option<String>,
}
