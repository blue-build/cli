/// Creates or modifies a `std::process::Command` adding args.
///
/// # Examples
/// ```
/// use blue_build_utils::cmd;
///
/// const NAME: &str = "Bob";
/// let mut command = cmd!("echo", "Hello world!");
/// cmd!(command, "This is Joe.", format!("And this is {NAME}"));
/// command.status().unwrap();
/// ```
#[macro_export]
macro_rules! cmd {
    ($command:ident, $($arg:expr),+ $(,)?) => {
        {
            $command$(.arg($arg))*;
        }
    };
    ($command:expr, $($arg:expr),+ $(,)?) => {
        {
            let mut c = cmd!($command);
            c$(.arg($arg))*;
            c
        }
    };
    ($command:expr) => {
        {
            ::std::process::Command::new($command)
        }
    };
}

/// Use a key-word-like syntax to add environment variables to
/// a `std::process::Command`.
///
/// # Examples
/// ```
/// use blue_build_utils::{cmd, cmd_env};
///
/// const TEST: &str = "TEST";
/// let mut command = cmd_env!("echo", TEST => "This is a test");
/// cmd_env!(command, "ANOTHER_TEST" => "This is yet another test");
/// cmd!(command, "Hello, this is a ${TEST}");
/// command.status().unwrap();
/// ```
#[macro_export]
macro_rules! cmd_env {
    ($command:ident, $($key:expr => $value:expr),* $(,)?) => {
        {
            $command$(.env($key, $value))*;
        }
    };
    ($command:expr, $($key:expr => $value:expr),* $(,)?) => {
        {
            let mut c = cmd!($command);
            c$(.env($key, $value))*;
            c
        }
    }
}

#[macro_export]
macro_rules! string {
    ($str:expr) => {
        String::from($str)
    };
}

#[macro_export]
macro_rules! string_vec {
    ($($string:expr),+ $(,)?) => {
        {
            use $crate::string;
            vec![
                $(string!($string),)*
            ]
        }
    };
}
