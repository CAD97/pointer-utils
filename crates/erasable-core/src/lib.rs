#![allow(clippy::missing_safety_doc, unused, clippy::multiple_bound_locations)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{fmt, marker::PhantomData, ops::Deref, ptr::NonNull};

/// A well-behaved pointer type which can round-trip through a raw pointer.
pub unsafe trait Ptr: Sized {
    /// The type which this pointer points to.
    type Pointee: ?Sized;

    /// Converts this pointer into a raw pointer.
    fn into_raw_ptr(this: Self) -> *mut Self::Pointee;
    /// Converts a raw pointer back into this pointer type.
    unsafe fn from_raw_ptr(this: *mut Self::Pointee) -> Self;
}

unsafe impl<T: ?Sized> Ptr for NonNull<T> {
    type Pointee = T;

    fn into_raw_ptr(this: Self) -> *mut T {
        this.as_ptr()
    }

    unsafe fn from_raw_ptr(this: *mut T) -> Self {
        NonNull::new_unchecked(this)
    }
}

/// A pointee type which can retype a type-erased pointer.
///
/// This trait is automatically implemented for all sized types. Unsized types
/// which can be used behind a thin pointer can implement this trait manually.
/// Unfortunately, coherence rules mean that generic types which are sometimes
/// unsized can only implement this trait for sized instantiations.
pub unsafe trait Erasable {
    /// Reconstruct a (potentially wide) pointer to `Self` from a type-erased pointer.
    ///
    /// # Safety
    ///
    /// If <code>Self: [Sized]</code>, this is just a pointer cast and is always safe.
    /// Otherwise, `this` must point to an actual instance of `Self` and be valid for
    /// whatever reads are required to reconstruct the wide pointer metadata.
    ///
    /// Guaranteeing that calling `retype_ptr` in a generic context is sound is quite
    /// subtle and interacts with as-of-yet undecided details about the Rust Abstract
    /// Machine memory and borrow model. For now, it is recommended to only call this
    /// function in contexts where `AnyPtr` is acting as a type-erased thin reference
    /// where performing the necessary reads should never deactivate other pointers.
    ///
    /// For the specific guarantees, see the trait impl safety documentation section.
    unsafe fn retype_ptr(this: AnyPtr) -> NonNull<Self>;
}

unsafe impl<T: Sized> Erasable for T {
    unsafe fn retype_ptr(this: AnyPtr) -> NonNull<Self> {
        this.raw.cast()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct AnyPtr {
    raw: NonNull<()>,
}

impl<P> From<P> for AnyPtr
where
    P: Ptr + Deref,
{
    fn from(ptr: P) -> Self {
        AnyPtr {
            raw: unsafe { NonNull::new_unchecked(P::into_raw_ptr(ptr).cast()) },
        }
    }
}

impl fmt::Debug for AnyPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.raw.as_ptr(), f)
    }
}

impl fmt::Pointer for AnyPtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.raw.as_ptr(), f)
    }
}

impl AnyPtr {
    pub unsafe fn into_typed<P>(self) -> P
    where
        P: Ptr,
        P::Pointee: Erasable,
    {
        P::from_raw_ptr(P::Pointee::retype_ptr(self).as_ptr())
    }

    pub unsafe fn as_ref<'a, T: ?Sized>(&self) -> &'a T
    where
        T: Erasable,
    {
        T::retype_ptr(*self).as_ref()
    }

    pub unsafe fn as_mut<'a, T: ?Sized>(&mut self) -> &'a mut T
    where
        T: Erasable,
    {
        T::retype_ptr(*self).as_mut()
    }

    pub const fn cast<T: Sized>(self) -> NonNull<T> {
        self.raw.cast()
    }
}

#[repr(transparent)]
pub struct Thin<P: Ptr>
where
    P::Pointee: Erasable,
{
    raw: AnyPtr,
    marker: PhantomData<P>,
}

impl<P: Ptr> Drop for Thin<P>
where
    P::Pointee: Erasable,
{
    fn drop(&mut self) {
        drop(unsafe { self.raw.into_typed::<P>() });
    }
}
