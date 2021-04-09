//! Simple implementation of `ElementStorage<T>`.

use core::{
    alloc::AllocError,
    fmt::{self, Debug},
    marker::Unsize,
    mem::MaybeUninit,
    ptr::{self, NonNull},
    marker::PhantomData,
    cell::UnsafeCell,
};

use rfc2580::{self, Pointee};

use crate::{traits::ElementStorage, utils};

pub struct NonTrackingElementHandle<T: ?Sized + Pointee, S> {
    data: UnsafeCell<MaybeUninit<S>>,
    meta: T::MetaData,
}

impl<T: ?Sized + Pointee, S> Debug for NonTrackingElementHandle<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NonTrackingElementHandle")
    }
}

#[derive(Debug)]
/// NonTrackingElement is an inline storage without tracking.
pub struct NonTrackingElement<S> {
    _marker: PhantomData<S>,
}

impl<S> ElementStorage for NonTrackingElement<S> {
    type Handle<T: ?Sized + Pointee> = NonTrackingElementHandle<T, S>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, _handle: &Self::Handle<T>) {
        // do nothing
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: &Self::Handle<T>) -> NonNull<T> {
        let ptr = NonNull::from(&handle.data).cast();

        rfc2580::from_non_null_parts(handle.meta, ptr)
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(
        &self,
        handle: &Self::Handle<T>,
    ) -> Self::Handle<U> {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.get(handle);

        let meta = rfc2580::into_raw_parts(element.as_ptr() as *mut U).0;

        let new_handle = NonTrackingElementHandle { data: UnsafeCell::new(MaybeUninit::uninit()), meta };
        ptr::copy_nonoverlapping::<MaybeUninit<S>>(handle.data.get(), new_handle.data.get(), 1);
        new_handle
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::MetaData,
    ) -> Result<Self::Handle<T>, AllocError> {
        let _ = utils::validate_layout::<T, S>(meta)?;

        Ok(NonTrackingElementHandle { data: UnsafeCell::new(MaybeUninit::uninit()), meta })
    }
}

impl<S> NonTrackingElement<S> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: PhantomData
        }
    }
}

impl<S> Default for NonTrackingElement<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn new_unconditional_success() {
        NonTrackingElement::<u8>::new();
    }

    #[test]
    fn create_success() {
        let mut storage = NonTrackingElement::<[u8; 2]>::new();
        storage.create(1u8).unwrap();
    }

    #[test]
    fn create_insufficient_size() {
        let mut storage = NonTrackingElement::<u8>::new();
        storage.create([1u8, 2, 3]).unwrap_err();
    }

    #[test]
    fn create_insufficient_alignment() {
        let mut storage = NonTrackingElement::<[u8; 32]>::new();
        storage.create([1u32]).unwrap_err();
    }

    #[test]
    fn coerce() {
        let mut storage = NonTrackingElement::<[u8; 32]>::new();

        let handle = storage.create([1u8, 2u8]).unwrap();

        //  Safety:
        //  -   `handle` is valid.
        let handle = unsafe { storage.coerce::<[u8], _>(&handle) };

        //  Safety:
        //  -   `handle` is valid.
        unsafe { storage.destroy(&handle) };
    }
} // mod tests
