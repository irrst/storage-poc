//! Fallback implementation of `ElementStorage`.

use core::{
    alloc::AllocError,
    fmt::{self, Debug},
    marker::Unsize,
    ptr::NonNull,
};

use rfc2580::Pointee;

use crate::traits::ElementStorage;

/// FallbackElement is a fallback implementation of 2 ElementStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct FallbackElement<F, S> {
    first: F,
    second: S,
}

impl<F, S> FallbackElement<F, S> {
    /// Creates an instance.
    pub fn new(first: F, second: S) -> Self {
        Self { first, second }
    }
}

impl<F, S> ElementStorage for FallbackElement<F, S>
where
    F: ElementStorage,
    S: ElementStorage,
{
    type Handle<T: ?Sized + Pointee> = FallbackElementHandle<F::Handle<T>, S::Handle<T>>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: &Self::Handle<T>) {
        use FallbackElementHandle::*;

        match handle {
            First(first) => self.first.deallocate(first),
            Second(second) => self.second.deallocate(second),
        }
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: &Self::Handle<T>) -> NonNull<T> {
        use FallbackElementHandle::*;

        match handle {
            First(first) => self.first.get(first),
            Second(second) => self.second.get(second),
        }
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(
        &self,
        handle: &Self::Handle<T>,
    ) -> Self::Handle<U> {
        use FallbackElementHandle::*;

        match handle {
            First(first) => First(self.first.coerce(first)),
            Second(second) => Second(self.second.coerce(second)),
        }
    }

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        use FallbackElementHandle::*;

        match self.first.create(value) {
            Ok(handle) => Ok(First(handle)),
            Err(value) => self.second.create(value).map(|handle| Second(handle)),
        }
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::MetaData,
    ) -> Result<Self::Handle<T>, AllocError> {
        use FallbackElementHandle::*;

        self.first
            .allocate::<T>(meta)
            .map(|handle| First(handle))
            .or_else(|_| self.second.allocate::<T>(meta).map(|handle| Second(handle)))
    }
}

impl<F, S> Debug for FallbackElement<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "FallbackElement")
    }
}

impl<F: Default, S: Default> Default for FallbackElement<F, S> {
    fn default() -> Self {
        Self::new(F::default(), S::default())
    }
}

/// FallbackElementHandle, an alternative between 2 handles.
pub enum FallbackElementHandle<F, S> {
    /// First storage handle.
    First(F),
    /// Second storage handle.
    Second(S),
}

impl<F, S> Debug for FallbackElementHandle<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "FallbackElementHandle")
    }
}
