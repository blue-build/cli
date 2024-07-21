/// Creates or modifies a `std::process::Command` adding
/// args or environment variables.
///
/// # Examples
/// ```
/// use blue_build_utils::cmd;
///
/// const NAME: &str = "Bob";
/// const TEST: &str = "TEST";
/// let mut command = cmd!("echo", "Hello world!");
/// cmd!(command, "This is Joe.");
/// cmd!(command, format!("And this is {NAME}"));
/// command.status().unwrap();
/// let mut command = cmd!("echo", "Is this a ${TEST}?"; TEST = "This is a test");
/// cmd!(command, "ANOTHER_TEST" = "This is yet another test");
/// command.status().unwrap();
/// ```
#[macro_export]
macro_rules! cmd {
    ($command:literal) => {
        {
            ::std::process::Command::new($command)
        }
    };
    ($command:literal, $($env_key:tt = $env_value:expr),+ $(; $($tail:tt)*)?) => {
        {
            let mut c = cmd!($command);
            c$(.env($env_key, $env_value))*;
            $(cmd!(c, $($tail)*);)*
            c
        }
    };
    ($command:literal, $($arg:expr),+ $(; $($tail:tt)*)?) => {
        {
            let mut c = cmd!($command);
            c$(.arg($arg))*;
            $(cmd!(c, $($tail)*);)*
            c
        }
    };
    ($command:ident, $($env_key:tt = $env_value:expr),+ $(; $($tail:tt)*)?) => {
        {
            $command$(.env($env_key, $env_value))*;
            $(cmd!($command, $($tail)*);)*
        }
    };
    ($command:ident, $($arg:expr),+ $(; $($tail:tt)*)?) => {
        {
            $command$(.arg($arg))*;
            $(cmd!($command, $($tail)*);)*
        }
    }
}
