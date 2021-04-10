//! Simple implementation of `ElementStorage<T>`.

use core::{
    alloc::{AllocError, Allocator, Layout},
    fmt::{self, Debug},
    marker::Unsize,
    mem::MaybeUninit,
    ptr::NonNull,
};

use rfc2580::{self, Pointee};

use crate::{
    alternative::Builder,
    traits::{ElementStorage, RangeStorage},
    utils,
};

use super::AllocatorBuilder;

/// Generic allocator-based ElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct AllocStorage<A> {
    allocator: A,
}

impl<A> AllocStorage<A> {
    /// Creates an instance of AllocStorage.
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }
}

impl<A: Allocator> ElementStorage for AllocStorage<A> {
    type Handle<T: ?Sized + Pointee> = NonNull<T>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: &Self::Handle<T>) {
        //  Safety:
        //  -   `element` points to a valid value.
        let layout = Layout::for_value_raw(handle.as_ptr());

        //  Safety:
        //  -   `element` was allocated by call to `self.allocator`.
        //  -   `layout` matches that of allocation.
        self.allocator.deallocate(handle.cast(), layout);
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: &Self::Handle<T>) -> NonNull<T> {
        handle.clone()
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(
        &self,
        handle: &Self::Handle<T>,
    ) -> Self::Handle<U> {
        handle.clone()
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::MetaData,
    ) -> Result<Self::Handle<T>, AllocError> {
        let slice = self.allocator.allocate(utils::layout_of::<T>(meta))?;

        let pointer: NonNull<u8> = slice.as_non_null_ptr().cast();

        Ok(rfc2580::from_non_null_parts(meta, pointer))
    }
}

impl<A: Allocator> RangeStorage for AllocStorage<A> {
    type Handle<T> = NonNull<[MaybeUninit<T>]>;

    type Capacity = usize;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        usize::MAX
    }

    unsafe fn deallocate<T>(&mut self, handle: &Self::Handle<T>) {
        if handle.len() > 0 {
            let layout = Self::layout_of(handle.clone());
            let pointer = Self::from_handle(handle.clone());
            self.allocator.deallocate(pointer, layout);
        }
    }

    unsafe fn get<T>(&self, handle: &Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        handle.clone()
    }

    unsafe fn try_grow<T>(
        &mut self,
        handle: &Self::Handle<T>,
        new_capacity: Self::Capacity,
    ) -> Result<Self::Handle<T>, AllocError> {
        debug_assert!(handle.len() < new_capacity);

        if handle.len() == 0 {
            return <Self as RangeStorage>::allocate::<T>(self, new_capacity);
        }

        let old_layout = Self::layout_of(handle.clone());
        let old_pointer = Self::from_handle(handle.clone());

        let new_layout = Self::layout_for::<T>(new_capacity)?;
        let new_pointer = self.allocator.grow(old_pointer, old_layout, new_layout)?;

        Ok(Self::into_handle(new_pointer, new_capacity))
    }

    unsafe fn try_shrink<T>(
        &mut self,
        handle: &Self::Handle<T>,
        new_capacity: Self::Capacity,
    ) -> Result<Self::Handle<T>, AllocError> {
        debug_assert!(handle.len() > new_capacity);

        if handle.len() == 0 {
            return Err(AllocError);
        }

        let old_layout = Self::layout_of(handle.clone());
        let old_pointer = Self::from_handle(handle.clone());

        if new_capacity == 0 {
            self.allocator.deallocate(old_pointer, old_layout);
            return Ok(Self::dangling_handle());
        }

        let new_layout = Self::layout_for::<T>(new_capacity)?;
        let new_pointer = self.allocator.shrink(old_pointer, old_layout, new_layout)?;

        Ok(Self::into_handle(new_pointer, new_capacity))
    }

    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        if capacity == 0 {
            return Ok(Self::dangling_handle());
        }

        let layout = Self::layout_for::<T>(capacity)?;
        let pointer = self.allocator.allocate(layout)?;
        Ok(Self::into_handle(pointer, capacity))
    }
}

impl<A: Allocator> Builder<AllocStorage<A>> for A {
    fn from_storage(storage: AllocStorage<A>) -> A {
        storage.allocator
    }

    fn into_storage(self) -> AllocStorage<A> {
        AllocStorage::new(self)
    }
}

impl<A> Builder<AllocStorage<A>> for AllocatorBuilder<A> {
    fn from_storage(storage: AllocStorage<A>) -> Self {
        AllocatorBuilder(storage.allocator)
    }

    fn into_storage(self) -> AllocStorage<A> {
        AllocStorage::new(self.0)
    }
}

impl<A: Default> Default for AllocStorage<A> {
    fn default() -> Self {
        let allocator = A::default();
        Self::new(allocator)
    }
}

impl<A> Debug for AllocStorage<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "AllocStorage")
    }
}

//
//  Implementation
//
impl<A: Allocator> AllocStorage<A> {
    fn dangling_handle<T>() -> NonNull<[MaybeUninit<T>]> {
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }

    fn layout_for<T>(capacity: usize) -> Result<Layout, AllocError> {
        debug_assert!(capacity > 0);

        Layout::array::<T>(capacity).map_err(|_| AllocError)
    }

    fn layout_of<T>(handle: NonNull<[MaybeUninit<T>]>) -> Layout {
        debug_assert!(handle.len() > 0);

        Layout::array::<T>(handle.len()).expect("Valid handle")
    }

    fn from_handle<T>(handle: NonNull<[MaybeUninit<T>]>) -> NonNull<u8> {
        debug_assert!(handle.len() > 0);

        handle.as_non_null_ptr().cast()
    }

    fn into_handle<T>(pointer: NonNull<[u8]>, capacity: usize) -> NonNull<[MaybeUninit<T>]> {
        NonNull::slice_from_raw_parts(pointer.as_non_null_ptr().cast(), capacity)
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::{NonAllocator, SpyAllocator};

    use super::*;

    // Element tests

    #[test]
    fn default_unconditional_success() {
        AllocStorage::<NonAllocator>::default();
    }

    #[test]
    fn new_unconditional_success() {
        AllocStorage::new(NonAllocator);
    }

    #[test]
    fn create_success() {
        let allocator = SpyAllocator::default();

        let mut storage = AllocStorage::new(allocator.clone());
        let handle = storage.create(1u32).unwrap();

        assert_eq!(1, allocator.allocated());
        assert_eq!(0, allocator.deallocated());

        unsafe { storage.destroy(&handle) };

        assert_eq!(1, allocator.allocated());
        assert_eq!(1, allocator.deallocated());
    }

    #[test]
    fn create_failure() {
        let mut storage = AllocStorage::new(NonAllocator);
        storage.create(1u8).unwrap_err();
    }

    #[test]
    fn coerce() {
        let allocator = SpyAllocator::default();

        let mut storage = AllocStorage::new(allocator.clone());
        let handle = storage.create([1u8, 2]).unwrap();

        assert_eq!(1, allocator.allocated());
        assert_eq!(0, allocator.deallocated());

        let handle = unsafe { storage.coerce::<[u8], _>(&handle) };

        assert_eq!([1, 2], unsafe {
            <_ as ElementStorage>::get(&storage, &handle).as_ref()
        });

        unsafe { storage.destroy(&handle) };

        assert_eq!(1, allocator.allocated());
        assert_eq!(1, allocator.deallocated());
    }

    #[test]
    fn coerce_success() {
        let allocator = SpyAllocator::default();

        let mut storage = AllocStorage::new(allocator.clone());
        let handle = storage.create([1u32, 2, 3]).unwrap();
        let handle = unsafe { storage.coerce::<[u32], _>(&handle) };

        assert_eq!(1, allocator.allocated());
        assert_eq!(0, allocator.deallocated());

        unsafe { storage.destroy(&handle) };

        assert_eq!(1, allocator.allocated());
        assert_eq!(1, allocator.deallocated());
    }

    // Range tests

    #[test]
    fn allocate_zero_success() {
        let mut storage = AllocStorage::new(NonAllocator);

        let slice = <_ as RangeStorage>::allocate::<String>(&mut storage, 0).unwrap();

        assert_eq!(0, slice.len());
    }

    #[test]
    fn allocate_success() {
        let allocator = SpyAllocator::default();

        let mut storage = AllocStorage::new(allocator.clone());
        let handle = <_ as RangeStorage>::allocate::<String>(&mut storage, 1).unwrap();

        assert_eq!(1, allocator.allocated());
        assert_eq!(0, allocator.deallocated());

        unsafe { <_ as RangeStorage>::deallocate(&mut storage, &handle) };

        assert_eq!(1, allocator.allocated());
        assert_eq!(1, allocator.deallocated());
    }

    #[test]
    fn allocate_failure() {
        let mut storage = AllocStorage::new(NonAllocator);
        <_ as RangeStorage>::allocate::<String>(&mut storage, 1).unwrap_err();
    }
} // mod tests
