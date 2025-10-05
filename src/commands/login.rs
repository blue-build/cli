use std::io::{self, Read};

use blue_build_process_management::drivers::{BuildDriver, Driver, DriverArgs, SigningDriver};
use blue_build_utils::{
    credentials::{Credentials, CredentialsArgs},
    secret::SecretValue,
};
use clap::Args;
use miette::{IntoDiagnostic, Result, bail};
use requestty::questions;

use super::BlueBuildCommand;

#[derive(Debug, Clone, Args)]
pub struct LoginCommand {
    /// The server to login to.
    server: String,

    /// The password to login with.
    ///
    /// Cannont be used with `--password-stdin`.
    #[arg(group = "pass", long, short)]
    password: Option<String>,

    /// Read password from stdin,
    ///
    /// Cannot be used with `--password/-p`
    #[arg(group = "pass", long)]
    password_stdin: bool,

    /// The username to login with
    #[arg(long, short)]
    username: Option<String>,

    #[clap(flatten)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for LoginCommand {
    fn try_run(&mut self) -> miette::Result<()> {
        Driver::init(self.drivers);

        Credentials::init(
            CredentialsArgs::builder()
                .registry(&self.server)
                .username(self.get_username()?)
                .password(self.get_password()?)
                .build(),
        );

        Driver::login(&self.server)?;
        Driver::signing_login(&self.server)?;

        Ok(())
    }
}

impl LoginCommand {
    fn get_username(&self) -> Result<String> {
        Ok(if let Some(ref username) = self.username {
            username.clone()
        } else if !self.password_stdin {
            let questions = questions! [ inline
                Input {
                    name: "username",
                },
            ];

            requestty::prompt(questions)
                .into_diagnostic()?
                .get("username")
                .unwrap()
                .as_string()
                .unwrap()
                .to_string()
        } else {
            bail!("Cannot prompt for username when using `--password-stdin`");
        })
    }

    fn get_password(&self) -> Result<SecretValue> {
        Ok(if let Some(ref password) = self.password {
            password.clone().into()
        } else if self.password_stdin {
            let mut password = String::new();
            io::stdin()
                .read_to_string(&mut password)
                .into_diagnostic()?;
            password.into()
        } else {
            let questions = questions! [ inline
                Password {
                    name: "password",
                }
            ];

            requestty::prompt(questions)
                .into_diagnostic()?
                .get("password")
                .unwrap()
                .as_string()
                .unwrap()
                .to_string()
                .into()
        })
    }
}
