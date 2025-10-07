use std::{
    collections::HashMap,
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
    get_env_var,
    secret::SecretValue,
    string,
};

static CREDS: LazyLock<Mutex<HashMap<String, Credentials>>> =
    LazyLock::new(|| Mutex::new(HashMap::default()));

/// The credentials for logging into image registries.
#[derive(Debug, Clone)]
pub enum Credentials {
    Basic {
        username: String,
        password: SecretValue,
    },
    Token(SecretValue),
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
        let mut creds = CREDS.lock().expect("Must lock CREDS");

        let CredentialsArgs {
            username,
            password,
            registry,
        } = args;

        let registry = match (
            registry,
            get_env_var(CI_REGISTRY).ok(),
            get_env_var(GITHUB_ACTIONS).ok(),
        ) {
            (Some(registry), _, _) | (_, Some(registry), _) if !registry.is_empty() => registry,
            (_, _, Some(_)) => string!("ghcr.io"),
            _ => return,
        };

        let cred = match (
            (username, password),
            docker_credential::get_credential(&registry).ok(),
            docker_credential::get_podman_credential(&registry).ok(),
            (
                get_env_var(CI_REGISTRY_USER).ok(),
                get_env_var(CI_REGISTRY_PASSWORD)
                    .ok()
                    .map(SecretValue::from),
            ),
            (
                get_env_var(GITHUB_ACTOR).ok(),
                get_env_var(GITHUB_TOKEN).ok().map(SecretValue::from),
            ),
        ) {
            ((Some(username), Some(password)), _, _, _, _) => Self::Basic { username, password },
            (_, Some(DockerCredential::UsernamePassword(username, password)), _, _, _)
            | (_, _, Some(DockerCredential::UsernamePassword(username, password)), _, _) => {
                Self::Basic {
                    username,
                    password: password.into(),
                }
            }
            (_, Some(DockerCredential::IdentityToken(token)), _, _, _)
            | (_, _, Some(DockerCredential::IdentityToken(token)), _, _) => {
                Self::Token(token.into())
            }
            (_, _, _, (Some(username), Some(password)), _)
            | (_, _, _, _, (Some(username), Some(password)))
                if !username.is_empty() && !password.is_empty() =>
            {
                Self::Basic { username, password }
            }
            _ => return,
        };

        let _ = creds.insert(registry, cred);
        drop(creds);
    }

    /// Get the credentials for the current set of actions.
    ///
    /// # Panics
    /// Will panic if it can't lock the mutex.
    #[must_use]
    pub fn get(registry: &str) -> Option<Self> {
        trace!("credentials::get({registry})");

        let mut creds = CREDS.lock().expect("Must lock CREDS");

        match (
            creds.get(registry),
            docker_credential::get_credential(registry).ok(),
            docker_credential::get_podman_credential(registry).ok(),
        ) {
            (Some(creds), _, _) => Some(creds.clone()),
            (None, Some(DockerCredential::IdentityToken(token)), _)
            | (None, None, Some(DockerCredential::IdentityToken(token))) => {
                let cred = Self::Token(SecretValue::from(token));
                let _ = creds.insert(registry.into(), cred.clone());
                drop(creds);
                Some(cred)
            }
            (None, Some(DockerCredential::UsernamePassword(username, password)), _)
            | (None, None, Some(DockerCredential::UsernamePassword(username, password))) => {
                let cred = Self::Basic {
                    username,
                    password: password.into(),
                };
                let _ = creds.insert(registry.into(), cred.clone());
                drop(creds);
                Some(cred)
            }
            (None, None, None) => None,
        }
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
    pub password: Option<SecretValue>,
}
