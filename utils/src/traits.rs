use std::{borrow::Cow, ffi::OsStr, path::Path};

trait PrivateTrait<T: ?Sized>: IntoIterator {}

impl<T, R> PrivateTrait<R> for T where T: IntoIterator {}

#[expect(private_bounds)]
pub trait CowCollecter<'a, IN, OUT>: PrivateTrait<IN>
where
    IN: ToOwned + ?Sized,
    OUT: ToOwned + ?Sized,
{
    fn collect_cow_vec(&'a self) -> Vec<Cow<'a, OUT>>;
}

impl<'a, T, R> CowCollecter<'a, R, R> for T
where
    T: AsRef<[R]> + IntoIterator,
    R: ToOwned,
{
    fn collect_cow_vec(&'a self) -> Vec<Cow<'a, R>> {
        self.as_ref().iter().map(Cow::Borrowed).collect()
    }
}

macro_rules! impl_cow_collector {
    ($type:ty) => {
        impl<'a, T, R> CowCollecter<'a, R, $type> for T
        where
            T: AsRef<[R]> + IntoIterator,
            R: AsRef<$type> + ToOwned + 'a,
        {
            fn collect_cow_vec(&'a self) -> Vec<Cow<'a, $type>> {
                self.as_ref()
                    .iter()
                    .map(|v| v.as_ref())
                    .map(Cow::from)
                    .collect()
            }
        }
    };
}

impl_cow_collector!(str);
impl_cow_collector!(Path);
impl_cow_collector!(OsStr);

#[expect(private_bounds)]
pub trait AsRefCollector<'a, IN, OUT>: PrivateTrait<IN>
where
    IN: ?Sized,
    OUT: ?Sized,
{
    fn collect_as_ref_vec(&'a self) -> Vec<&'a OUT>;
}

impl<'a, T, R> AsRefCollector<'a, R, R> for T
where
    T: AsRef<[R]> + IntoIterator,
{
    fn collect_as_ref_vec(&'a self) -> Vec<&'a R> {
        self.as_ref().iter().collect()
    }
}

macro_rules! impl_asref_collector {
    ($type:ty) => {
        impl<'a, T, R> AsRefCollector<'a, R, $type> for T
        where
            T: AsRef<[R]> + IntoIterator,
            R: AsRef<$type> + 'a,
        {
            fn collect_as_ref_vec(&'a self) -> Vec<&'a $type> {
                self.as_ref().iter().map(AsRef::as_ref).collect()
            }
        }
    };
}

impl_asref_collector!(str);
impl_asref_collector!(Path);
impl_asref_collector!(OsStr);

#[expect(private_bounds)]
pub trait IntoCollector<IN, OUT>: PrivateTrait<IN>
where
    IN: Into<OUT>,
{
    fn collect_into_vec(self) -> Vec<OUT>;
}

impl<T, U, R> IntoCollector<U, R> for T
where
    T: IntoIterator<Item = U>,
    U: Into<R>,
{
    fn collect_into_vec(self) -> Vec<R> {
        self.into_iter().map(Into::into).collect()
    }
}
