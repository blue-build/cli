use std::{
    collections::HashSet,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Not,
    path::PathBuf,
};

use cached::proc_macro::cached;
use comlexr::cmd;
use miette::{Context, IntoDiagnostic, Result, bail};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use zeroize::Zeroizing;

use crate::{BUILD_ID, string};

mod private {
    pub trait Private {}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Secret {
    #[serde(rename = "env")]
    Env { name: String },
    #[serde(rename = "file")]
    File {
        source: PathBuf,
        destination: PathBuf,
    },
    #[serde(rename = "exec")]
    Exec(SecretExec),
    #[serde(rename = "ssh")]
    Ssh,
}

impl Secret {
    #[must_use]
    pub fn get_hash(&self) -> String {
        get_hash(self)
    }

    #[must_use]
    pub fn mount(&self) -> String {
        let hash = self.get_hash();
        let prefix = format!("--mount=type=secret,id={hash}");
        match self {
            Self::Env { name: _ }
            | Self::Exec(SecretExec {
                command: _,
                args: _,
                output: SecretExecOutput::Env { name: _ },
            }) => format!("{prefix},dst=/tmp/secrets/{hash}"),
            Self::File {
                source: _,
                destination,
            }
            | Self::Exec(SecretExec {
                command: _,
                args: _,
                output: SecretExecOutput::File { destination },
            }) => format!("{prefix},dst={}", destination.display()),
            Self::Ssh => string!("--ssh"),
        }
    }

    #[must_use]
    pub fn env(&self) -> Option<String> {
        let hash = self.get_hash();
        match self {
            Self::Env { name }
            | Self::Exec(SecretExec {
                command: _,
                args: _,
                output: SecretExecOutput::Env { name },
            }) => Some(format!(r#"{name}="$(cat /tmp/secrets/{hash})""#)),
            _ => None,
        }
    }
}

#[cached(key = "Secret", convert = "{secret.clone()}", sync_writes = "by_key")]
fn get_hash(secret: &Secret) -> String {
    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    BUILD_ID.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

impl private::Private for Vec<Secret> {}

pub trait SecretMounts: private::Private {
    fn mounts(&self) -> Vec<String>;
    fn envs(&self) -> Vec<String>;
}

impl SecretMounts for Vec<Secret> {
    fn mounts(&self) -> Vec<String> {
        self.iter().map(Secret::mount).collect()
    }

    fn envs(&self) -> Vec<String> {
        self.iter().filter_map(Secret::env).collect()
    }
}

impl<H: std::hash::BuildHasher> private::Private for HashSet<&Secret, H> {}

#[allow(private_bounds)]
pub trait SecretArgs: private::Private {
    /// Retrieves the args for the image builder.
    ///
    /// If exec based secrets are included, will run the commands
    /// to put the results into files for mounting.
    ///
    /// # Errors
    /// Will error if an exec based secret fails to run.
    fn args(&self, temp_dir: &TempDir) -> Result<Vec<String>>;
}

impl<H: std::hash::BuildHasher> SecretArgs for HashSet<&Secret, H> {
    fn args(&self, temp_dir: &TempDir) -> Result<Vec<String>> {
        self.iter()
            .map(|secret| {
                Ok(match secret {
                    Secret::Env { name } => {
                        format!(
                            "--secret=id={},type=env,src={}",
                            secret.get_hash(),
                            name.trim()
                        )
                    }
                    Secret::File {
                        source,
                        destination: _,
                    } => {
                        format!(
                            "--secret=id={},type=file,src={}",
                            secret.get_hash(),
                            source.display()
                        )
                    }
                    Secret::Exec(exec) => {
                        let result = exec.exec()?;
                        let hash = secret.get_hash();
                        let secret_path = temp_dir.path().join(&hash);
                        fs::write(&secret_path, result.value())
                            .into_diagnostic()
                            .wrap_err("Failed to write secret to temp file")?;
                        format!("--secret=id={hash},src={}", secret_path.display())
                    }
                    Secret::Ssh => string!("--ssh"),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SecretExec {
    pub command: String,
    pub args: Vec<String>,
    pub output: SecretExecOutput,
}

impl SecretExec {
    /// Executes the command to retrieve the secret value.
    ///
    /// # Errors
    /// Will error if the command fails to execute.
    pub fn exec(&self) -> Result<SecretValue> {
        let output = cmd!(&self.command, for &self.args)
            .output()
            .into_diagnostic()
            .wrap_err_with(|| format!("Unable to execute `{}`", self.command))?;

        if output.status.success().not() {
            bail!("Failed to execute `{}` to retrieve secret", self.command);
        }

        String::from_utf8(output.stdout)
            .map(SecretValue::from)
            .into_diagnostic()
            .wrap_err_with(|| "Failed to read output")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SecretExecOutput {
    #[serde(rename = "env")]
    Env { name: String },
    #[serde(rename = "file")]
    File { destination: PathBuf },
}

#[derive(Deserialize)]
pub struct SecretValue(Zeroizing<String>);

macro_rules! impl_secret_value {
    ($($type:ty),*) => {
        $(
            impl From<$type> for SecretValue {
                fn from(value: $type) -> Self {
                    Self(String::from(value.trim()).into())
                }
            }
        )*
    };
}

impl_secret_value!(String, &String, &str);

impl SecretValue {
    /// Get the value of the secret.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}
