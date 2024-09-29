use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

trait PrivateTrait<T: ToOwned + ?Sized> {}

macro_rules! impl_private_trait {
    ($lt:lifetime, $type:ty) => {
        impl<$lt, T> PrivateTrait<$type> for T where T: AsRef<[&$lt $type]> {}
    };
    ($type:ty) => {
        impl<T> PrivateTrait<$type> for T where T: AsRef<[$type]> {}
    };
}

impl_private_trait!(String);
impl_private_trait!('a, str);
impl_private_trait!(PathBuf);
impl_private_trait!('a, Path);
impl_private_trait!(OsString);
impl_private_trait!('a, OsStr);

#[allow(private_bounds)]
pub trait CowCollecter<'a, IN, OUT>: PrivateTrait<IN>
where
    IN: ToOwned + ?Sized,
    OUT: ToOwned + ?Sized,
{
    fn to_cow_vec(&'a self) -> Vec<Cow<'a, OUT>>;
}

macro_rules! impl_cow_collector {
    ($type:ty) => {
        impl<'a, T> CowCollecter<'a, $type, $type> for T
        where
            T: AsRef<[&'a $type]>,
        {
            fn to_cow_vec(&'a self) -> Vec<Cow<'a, $type>> {
                self.as_ref().iter().map(|v| Cow::Borrowed(*v)).collect()
            }
        }
    };
    ($in:ty, $out:ty) => {
        impl<'a, T> CowCollecter<'a, $in, $out> for T
        where
            T: AsRef<[$in]>,
        {
            fn to_cow_vec(&'a self) -> Vec<Cow<'a, $out>> {
                self.as_ref().iter().map(Cow::from).collect()
            }
        }
    };
}

impl_cow_collector!(String, str);
impl_cow_collector!(str);
impl_cow_collector!(PathBuf, Path);
impl_cow_collector!(Path);
impl_cow_collector!(OsString, OsStr);
impl_cow_collector!(OsStr);
