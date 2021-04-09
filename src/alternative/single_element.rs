//! Alternative implementation of `ElementStorage`.

use core::{
    alloc::AllocError,
    fmt::{self, Debug},
    hint,
    marker::Unsize,
    mem,
    ptr::NonNull,
    mem::ManuallyDrop,
};

use rfc2580::Pointee;

use crate::traits::ElementStorage;

use super::{Builder, Inner};

/// SingleElement is a composite of 2 ElementStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct SingleElement<F, S, FB, SB>(Inner<F, S, FB, SB>);

impl<F, S, FB, SB> SingleElement<F, S, FB, SB> {
    /// Creates an instance containing the First alternative.
    pub fn first(first: F, second_builder: SB) -> Self {
        Self(Inner::first(first, second_builder))
    }

    /// Creates an instance containing the Second alternative.
    pub fn second(second: S, first_builder: FB) -> Self {
        Self(Inner::second(second, first_builder))
    }
}

impl<F, S, FB, SB> ElementStorage for SingleElement<F, S, FB, SB>
where
    F: ElementStorage,
    S: ElementStorage,
    FB: Builder<F>,
    SB: Builder<S>,
{
    type Handle<T: ?Sized + Pointee> = SingleElementHandle<F::Handle<T>, S::Handle<T>>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: &Self::Handle<T>) {
        match &mut self.0 {
            Inner::First(ref mut first) => first.deallocate(&handle.first),
            Inner::Second(ref mut second) => second.deallocate(&handle.second),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: &Self::Handle<T>) -> NonNull<T> {
        match &self.0 {
            Inner::First(ref first) => first.get(&handle.first),
            Inner::Second(ref second) => second.get(&handle.second),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(
        &self,
        handle: &Self::Handle<T>,
    ) -> Self::Handle<U> {
        match &self.0 {
            Inner::First(ref first) => SingleElementHandle {
                first: ManuallyDrop::new(first.coerce(&handle.first)),
            },
            Inner::Second(ref second) => SingleElementHandle {
                second: ManuallyDrop::new(second.coerce(&handle.second)),
            },
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        match &mut self.0 {
            Inner::First(ref mut first) => match first.create(value) {
                Ok(first) => Ok(SingleElementHandle { first: ManuallyDrop::new(first) }),
                Err(value) => {
                    if let Inner::First(first) = mem::replace(&mut self.0, Inner::Poisoned) {
                        let (second, result) = first.transform(|_, second: &mut S| {
                            second
                                .create(value)
                                .map(|second| SingleElementHandle { second: ManuallyDrop::new(second) })
                        });
                        self.0 = Inner::Second(second);
                        return result;
                    }
                    //  Safety:
                    //  -   self.0 was First before invoking replace, hence replace returns First.
                    unsafe { hint::unreachable_unchecked() };
                }
            },
            Inner::Second(ref mut second) => second
                .create(value)
                .map(|second| SingleElementHandle { second: ManuallyDrop::new(second) }),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::MetaData,
    ) -> Result<Self::Handle<T>, AllocError> {
        match &mut self.0 {
            Inner::First(ref mut first) => match first.allocate(meta) {
                Ok(first) => Ok(SingleElementHandle { first: ManuallyDrop::new(first) }),
                Err(_) => {
                    if let Inner::First(first) = mem::replace(&mut self.0, Inner::Poisoned) {
                        let (second, result) = first.transform(|_, second: &mut S| {
                            second
                                .allocate(meta)
                                .map(|second| SingleElementHandle { second: ManuallyDrop::new(second) })
                        });
                        self.0 = Inner::Second(second);
                        return result;
                    }
                    //  Safety:
                    //  -   self.0 was First before invoking replace, hence replace returns First.
                    unsafe { hint::unreachable_unchecked() };
                }
            },
            Inner::Second(ref mut second) => second
                .allocate(meta)
                .map(|second| SingleElementHandle { second: ManuallyDrop::new(second) }),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }
}

impl<F, S, FB, SB> Debug for SingleElement<F, S, FB, SB> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

impl<F: Default, S, FB, SB: Default> Default for SingleElement<F, S, FB, SB> {
    fn default() -> Self {
        Self(Inner::default())
    }
}

/// SingleElementHandle, an alternative between 2 handles.
pub union SingleElementHandle<F, S> {
    first: ManuallyDrop<F>,
    second: ManuallyDrop<S>,
}

impl<F, S> Debug for SingleElementHandle<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElementHandle")
    }
}
