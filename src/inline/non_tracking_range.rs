//! Simple implementation of `RangeStorage`.

use core::{
    alloc::AllocError,
    cmp,
    fmt::{self, Debug},
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr::NonNull,
    cell::UnsafeCell,
};

use crate::{
    traits::{Capacity, RangeStorage},
    utils,
};

pub struct NonTrackingRangeHandle<T, S, const N: usize> {
    data: UnsafeCell<[MaybeUninit<S>; N]>,
    _marker: PhantomData<T>,
}

/// NonTrackingRange is an inline storage without tracking.
pub struct NonTrackingRange<C, S, const N: usize> {
    _marker: PhantomData<(C, S)>
}

impl<C: Capacity, S, const N: usize> NonTrackingRange<C, S, N> {
    pub(crate) fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<C: Capacity, S, const N: usize> RangeStorage for NonTrackingRange<C, S, N> {
    type Handle<T> = NonTrackingRangeHandle<T, S, N>;

    type Capacity = C;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        assert!(mem::size_of::<S>().checked_mul(N).is_some());

        //  The maximum capacity cannot exceed what can fit in an `isize`.
        let capacity = cmp::min(C::max().into_usize(), N);

        C::from_usize(mem::size_of::<S>() * capacity / mem::size_of::<T>())
            .or_else(|| C::from_usize(capacity))
            .expect("Cannot fail, since capacity <= C::max()")
    }

    unsafe fn deallocate<T>(&mut self, _handle: &Self::Handle<T>) {
        // do nothing
    }

    unsafe fn get<T>(&self, handle: &Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        let pointer: NonNull<MaybeUninit<T>> = NonNull::from(&handle.data).cast();

        NonNull::slice_from_raw_parts(pointer, N)
    }

    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        utils::validate_array_layout::<T, [MaybeUninit<S>; N]>(capacity.into_usize())?;
        Ok(NonTrackingRangeHandle { data: UnsafeCell::new(MaybeUninit::uninit_array()), _marker: PhantomData})
    }
}

impl<C: Capacity, S, const N: usize> Default for NonTrackingRange<C, S, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S, const N: usize> Debug for NonTrackingRangeHandle<T, S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "NonTrackingRangeHandle")
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn new_unconditional_success() {
        NonTrackingRange::<u8, u8, 42>::new();
    }

    #[test]
    fn allocate_success() {
        let mut storage = NonTrackingRange::<u8, u8, 42>::new();
        storage.allocate::<u8>(2).unwrap();
    }

    #[test]
    fn allocate_insufficient_size() {
        let mut storage = NonTrackingRange::<u8, u8, 2>::new();
        storage.allocate::<u8>(3).unwrap_err();
    }

    #[test]
    fn allocate_insufficient_alignment() {
        let mut storage = NonTrackingRange::<u8, u8, 42>::new();
        storage.allocate::<u32>(1).unwrap_err();
    }
} // mod tests
