use core::fmt;

pub trait Pointer<T>
    where
        Self: Sized
        + Copy
        + Clone
        + fmt::Debug
        + core::hash::Hash
        + core::cmp::PartialEq
        + core::cmp::PartialOrd
        + core::ops::Deref<Target=T>
        + core::ops::DerefMut<Target=T>,
{
    const NULL: Self;

    fn dangling() -> Self;
    fn as_ptr(&self) -> *const T;
    fn as_mut_ptr(&self) -> *mut T;
    fn upcast(self) -> super::RawPtr;
    fn is_null(self) -> bool;
}
