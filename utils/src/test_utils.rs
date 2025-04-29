use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
    thread::{self, ThreadId},
};

use miette::{Result, miette};

use crate::string;

#[allow(clippy::type_complexity)]
static ENV_VARS: LazyLock<Arc<RwLock<HashMap<(ThreadId, String), String>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Test harness function for getting env variables.
///
/// # Errors
/// Will error if the env variable doesn't exist.
pub fn get_env_var<S>(key: S) -> Result<String>
where
    S: AsRef<str>,
{
    fn inner(key: &str) -> Result<String> {
        let thr_id = thread::current().id();

        let env_vars = ENV_VARS.read().unwrap();
        let key = (thr_id, string!(key));

        env_vars
            .get(&key)
            .map(ToOwned::to_owned)
            .inspect(|val| eprintln!("get: {key:?} = {val}"))
            .ok_or_else(|| miette!("Failed to retrieve env var '{key:?}'"))
    }
    inner(key.as_ref())
}

pub fn set_env_var<S, T>(key: S, value: T)
where
    S: AsRef<str>,
    T: AsRef<str>,
{
    fn inner(key: &str, value: &str) {
        let thr_id = thread::current().id();

        let mut env_vars = ENV_VARS.write().unwrap();

        let key = (thr_id, string!(key));
        eprintln!("set: {key:?} = {value}");

        env_vars
            .entry(key)
            .and_modify(|val| {
                *val = string!(value);
            })
            .or_insert_with(|| string!(value));
    }
    inner(key.as_ref(), value.as_ref());
}
