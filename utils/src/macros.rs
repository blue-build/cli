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
