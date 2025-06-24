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
