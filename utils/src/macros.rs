/// Easily create a `String`.
#[macro_export]
macro_rules! string {
    ($str:expr) => {
        String::from($str)
    };
}

/// Easily create a `Cow<'_, str>`.
#[macro_export]
macro_rules! cowstr {
    ($str:expr) => {
        ::std::borrow::Cow::<'_, str>::from($str)
    };
}

/// Easily create a `Vec<String>`.
/// Uses the same syntax as `vec![]`.
#[macro_export]
macro_rules! string_vec {
    ($($string:expr),* $(,)?) => {
        {
            vec![
                $($crate::string!($string),)*
            ]
        }
    };
}

/// Easily create a `Vec<Cow<'_, str>>`.
/// Uses the same syntax as `vec![]`.
#[macro_export]
macro_rules! cowstr_vec {
    ($($string:expr),* $(,)?) => {
        {
            vec![
                $($crate::cowstr!($string),)*
            ]
        }
    };
}

#[macro_export]
macro_rules! impl_de_fromstr {
    ($($typ:ty),* $(,)?) => {
        $(
            impl TryFrom<&str> for $typ {
                type Error = miette::Error;

                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    value.parse()
                }
            }

            impl TryFrom<&String> for $typ {
                type Error = miette::Error;

                fn try_from(value: &String) -> Result<Self, Self::Error> {
                    Self::try_from(value.as_str())
                }
            }

            impl TryFrom<String> for $typ {
                type Error = miette::Error;

                fn try_from(value: String) -> Result<Self, Self::Error> {
                    Self::try_from(value.as_str())
                }
            }

            impl<'de> serde::de::Deserialize<'de> for $typ {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    Self::try_from(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
                }
            }
        )*
    };
}

#[macro_export]
macro_rules! sudo_cmd {
    (
        prompt = $prompt:expr,
        sudo_check = $sudo_check:expr,
        $command:expr,
        $($rest:tt)*
    ) => {
        {
            let _use_sudo = ($sudo_check) && !$crate::running_as_root();

            ::comlexr::cmd!(
                if _use_sudo {
                    "sudo"
                } else {
                    $command
                },
                if _use_sudo && $crate::has_env_var($crate::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    $prompt,
                ],
                if _use_sudo => [
                    "--preserve-env",
                    $command,
                ],
                $($rest)*
            )
        }
    };
    (
        sudo_check = $sudo_check:expr,
        $command:expr,
        $($rest:tt)*
    ) => {
        {
            let _use_sudo = ($sudo_check) && !$crate::running_as_root();

            ::comlexr::cmd!(
                if _use_sudo {
                    "sudo"
                } else {
                    $command
                },
                if _use_sudo && $crate::has_env_var($crate::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    $crate::constants::SUDO_PROMPT,
                ],
                if _use_sudo => [
                    "--preserve-env",
                    $command,
                ],
                $($rest)*
            )
        }
    };
    (
        prompt = $prompt:expr,
        $command:expr,
        $($rest:tt)*
    ) => {
        {
            let _use_sudo = !$crate::running_as_root();

            ::comlexr::cmd!(
                if _use_sudo {
                    "sudo"
                } else {
                    $command
                },
                if _use_sudo && $crate::has_env_var($crate::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    $prompt,
                ],
                if _use_sudo => [
                    "--preserve-env",
                    $command,
                ],
                $($rest)*
            )
        }
    };
    (
        $command:expr,
        $($rest:tt)*
    ) => {
        {
            let _use_sudo = !$crate::running_as_root();

            ::comlexr::cmd!(
                if _use_sudo {
                    "sudo"
                } else {
                    $command
                },
                if _use_sudo && $crate::has_env_var($crate::constants::SUDO_ASKPASS) => [
                    "-A",
                    "-p",
                    $crate::constants::SUDO_PROMPT,
                ],
                if _use_sudo => [
                    "--preserve-env",
                    $command,
                ],
                $($rest)*
            )
        }
    };
}

#[macro_export]
macro_rules! cmd_out {
    (parse = String; err_msg = $err:expr; $($cmd:tt)*) => {
        $crate::cmd_out!(
            @start
            $err;
            |output| {
                $crate::cmd_out!(
                    @check
                    output;
                    $err;
                    ::std::string::String::from_utf8(output.stdout)
                        .into_diagnostic()
                        .wrap_err("When reading to string")
                        .wrap_err_with(|| $err)
                )
            };
            $($cmd)*
        )
    };
    (parse = $out_typ:ty; err_msg = $err:expr; $($cmd:tt)*) => {
        $crate::cmd_out!(
            @start
            $err;
            |output| {
                $crate::cmd_out!(
                    @check
                    output;
                    $err;
                    ::std::string::String::from_utf8(output.stdout)
                        .into_diagnostic()
                        .wrap_err("When reading to string")
                        .wrap_err_with(|| $err)
                        .and_then(|output| {
                            output.parse::<$out_typ>()
                                .into_diagnostic()
                                .wrap_err("When parsing")
                                .wrap_err_with(|| $err)
                        })
                )
            };
            $($cmd)*
        )
    };
    (from_json = $out_typ:ty; err_msg = $err:expr; $($cmd:tt)*) => {
        $crate::cmd_out!(
            @start
            $err;
            |output| {
                $crate::cmd_out!(
                    @check
                    output;
                    $err;
                    ::serde_json::from_slice::<$out_typ>(&output.stdout)
                        .into_diagnostic()
                        .wrap_err("When deserializing")
                        .wrap_err_with(|| $err)
                )
            };
            $($cmd)*
        )
    };
    (err_msg = $err:expr; $($cmd:tt)*) => {
        $crate::cmd_out!(
            @start
            $err;
            |output| {
                $crate::cmd_out!(
                    @check
                    output;
                    $err;
                    Ok(())
                )
            };
            $($cmd)*
        )
    };
    (@check $output:ident; $err:expr; $map:expr) => {
        if !$output.status.success() {
            Err(::miette::miette!(
                "{}\n{}",
                $err,
                ::std::string::String::from_utf8_lossy(&$output.stderr)
            ))
        } else {
            $map
        }
    };
    (@start $err:expr; $and_then:expr; $($cmd:tt)*) => {
        {
            use ::miette::{Context, IntoDiagnostic};
            {
                let _c = ::comlexr::cmd!($($cmd)*);
                ::log::trace!("{_c:#?}");
                dbg!(&_c);
                _c
            }
                .output()
                .into_diagnostic()
                .wrap_err("When calling the command")
                .wrap_err_with(|| $err)
                .and_then($and_then)
        }
    }
}
