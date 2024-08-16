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
    ($command:expr) => {
        ::std::process::Command::new($command)
    };
    ($command:ident, $($tail:tt)*) => {
        $crate::cmd!(@ $command, $($tail)*)
    };
    ($command:expr, $($tail:tt)*) => {
        {
            let mut c = $crate::cmd!($command);
            $crate::cmd!(@ c, $($tail)*);
            c
        }
    };
    (@ $command:ident $(,)?) => { };
    (@ $command:ident, for $for_expr:expr $(, $($tail:tt)*)?) => {
        {
            for arg in $for_expr.iter() {
                $crate::cmd!($command, arg);
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, for $iter:ident in $for_expr:expr => [ $($arg:expr),* $(,)? ] $(, $($tail:tt)*)?) => {
        {
            for $iter in $for_expr.iter() {
                $($crate::cmd!(@ $command, $arg);)*
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, for $iter:ident in $for_expr:expr => $arg:expr $(, $($tail:tt)*)?) => {
        {
            for $iter in $for_expr.iter() {
                $crate::cmd!(@ $command, $arg);
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, if let $let_pat:pat = $if_expr:expr => [ $($arg:expr),* $(,)? ] $(, $($tail:tt)*)?) => {
        {
            if let $let_pat = $if_expr {
                $($crate::cmd!(@ $command, $arg);)*
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, if let $let_pat:pat = $if_expr:expr => $arg:expr $(, $($tail:tt)*)?) => {
        {
            if let $let_pat = $if_expr {
                $crate::cmd!(@ $command, $arg);
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, if $if_expr:expr => [ $($arg:expr),* $(,)?] $(, $($tail:tt)*)?) => {
        {
            if $if_expr {
                $($crate::cmd!(@ $command, $arg);)*
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, if $if_expr:expr => $arg:expr $(, $($tail:tt)*)?) => {
        {
            if $if_expr {
                $crate::cmd!(@ $command, $arg);
            }
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, |$cmd_ref:ident|? $op:block $(, $($tail:tt)*)?) => {
        {
            let op_fn = |$cmd_ref: &mut ::std::process::Command| -> Result<()>  {
                $op
                Ok(())
            };
            op_fn(&mut $command)?;
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, |$cmd_ref:ident| $op:block $(, $($tail:tt)*)?) => {
        {
            let op_fn = |$cmd_ref: &mut ::std::process::Command| $op;
            op_fn(&mut $command);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, $key:expr => $value:expr $(, $($tail:tt)*)?) => {
        {
            $command.env($key, $value);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, current_dir = $dir:expr $(, $($tail:tt)*)?) => {
        {
            $command.current_dir($dir);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, stdin = $pipe:expr $(, $($tail:tt)*)?) => {
        {
            $command.stdin($pipe);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, stdout = $pipe:expr $(, $($tail:tt)*)?) => {
        {
            $command.stdout($pipe);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, stderr = $pipe:expr $(, $($tail:tt)*)?) => {
        {
            $command.stderr($pipe);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
    (@ $command:ident, $arg:expr $(, $($tail:tt)*)?) => {
        {
            $command.arg($arg);
            $($crate::cmd!(@ $command, $($tail)*);)*
        }
    };
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
                $($crate::string!($string),)*
            ]
        }
    };
}
