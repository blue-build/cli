use std::{env, sync::Mutex};

use blue_build_utils::constants::{
    BB_PASSWORD, BB_REGISTRY, BB_USERNAME, CI_REGISTRY, CI_REGISTRY_PASSWORD, CI_REGISTRY_USER,
    GITHUB_ACTIONS, GITHUB_ACTOR, GITHUB_TOKEN,
};
use clap::Args;
use log::trace;
use once_cell::sync::Lazy;
use typed_builder::TypedBuilder;

static INIT: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

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
static ENV_CREDENTIALS: Lazy<Option<Credentials>> = Lazy::new(|| {
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
        (Some(registry), _, _) if !registry.is_empty() => registry,
        (None, Some(ci_registry), None) if !ci_registry.is_empty() => ci_registry,
        (None, None, Some(_)) => "ghcr.io".to_string(),
        _ => return None,
    };
    trace!("Registry: {registry}");

    let username = match (
        username,
        env::var(CI_REGISTRY_USER).ok(),
        env::var(GITHUB_ACTOR).ok(),
    ) {
        (Some(username), _, _) if !username.is_empty() => username,
        (None, Some(ci_registry_user), None) if !ci_registry_user.is_empty() => ci_registry_user,
        (None, None, Some(github_actor)) if !github_actor.is_empty() => github_actor,
        _ => return None,
    };
    trace!("Username: {username}");

    let password = match (
        password,
        env::var(CI_REGISTRY_PASSWORD).ok(),
        env::var(GITHUB_TOKEN).ok(),
    ) {
        (Some(password), _, _) if !password.is_empty() => password,
        (None, Some(ci_registry_password), None) if !ci_registry_password.is_empty() => {
            ci_registry_password
        }
        (None, None, Some(registry_token)) if !registry_token.is_empty() => registry_token,
        _ => return None,
    };

    Some(
        Credentials::builder()
            .registry(registry)
            .username(username)
            .password(password)
            .build(),
    )
});

/// The credentials for logging into image registries.
#[derive(Debug, Default, Clone, TypedBuilder)]
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
            creds_lock.username = args.username;
            creds_lock.password = args.password;
            creds_lock.registry = args.registry;
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

#[derive(Debug, Default, Clone, TypedBuilder, Args)]
pub struct CredentialsArgs {
    /// The registry's domain name.
    #[arg(long, env = BB_REGISTRY)]
    #[builder(default, setter(into, strip_option))]
    pub registry: Option<String>,

    /// The username to login to the
    /// container registry.
    #[arg(short = 'U', long, env = BB_USERNAME, hide_env_values = true)]
    #[builder(default, setter(into, strip_option))]
    pub username: Option<String>,

    /// The password to login to the
    /// container registry.
    #[arg(short = 'P', long, env = BB_PASSWORD, hide_env_values = true)]
    #[builder(default, setter(into, strip_option))]
    pub password: Option<String>,
}

impl CredentialsArgs {}

/// Get the credentials for the current set of actions.
pub fn get() -> Option<&'static Credentials> {
    trace!("credentials::get()");
    ENV_CREDENTIALS.as_ref()
}
