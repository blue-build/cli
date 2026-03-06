use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum OrList<T> {
    Single(T),
    List(Vec<T>),
}

impl<T> From<T> for OrList<T> {
    fn from(value: T) -> Self {
        Self::Single(value)
    }
}

impl<T> From<&T> for OrList<T>
where
    T: ToOwned<Owned = T>,
{
    fn from(value: &T) -> Self {
        Self::Single(value.to_owned())
    }
}

impl<T> From<Vec<T>> for OrList<T> {
    fn from(value: Vec<T>) -> Self {
        Self::List(value)
    }
}

impl<T> From<&[T]> for OrList<T>
where
    T: Clone,
{
    fn from(value: &[T]) -> Self {
        Self::List(value.to_vec())
    }
}

macro_rules! impl_from_or_list {
    ($from:ty, $to:ty) => {
        impl From<$from> for OrList<$to> {
            fn from(value: $from) -> Self {
                Self::Single(value.into())
            }
        }

        impl From<Vec<$from>> for OrList<$to> {
            fn from(value: Vec<$from>) -> Self {
                Self::List(value.into_iter().map(Into::into).collect())
            }
        }
    };
}

impl_from_or_list!(&str, String);
impl_from_or_list!(&str, PathBuf);
impl_from_or_list!(String, PathBuf);
impl_from_or_list!(&Path, PathBuf);
