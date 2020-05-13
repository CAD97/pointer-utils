// NB: Last updated for Rust 1.40 parity. All impls are in rustdoc gutter order.

//! Borrowed forms of [`Rc`] and [`Arc`].
//!
//! [`ArcBorrow<_>`](`ArcBorrow`) is functionally equivalent to `&Arc<_>`,
//! but it's represented as `&T`, avoiding the extra indirection.
//!
//! # Examples
//!
//! ```rust
//! # use {rc_borrow::*, std::sync::Arc};
//! # type Resource = u32;
//! # fn acquire_resource() -> Arc<u32> { Arc::new(0) }
//! let resource: Arc<Resource> = acquire_resource();
//! let borrowed: ArcBorrow<'_, Resource> = (&resource).into();
//! let reference: &Resource = ArcBorrow::downgrade(borrowed);
//! let cloned: Arc<Resource> = ArcBorrow::upgrade(borrowed);
//! fn use_resource(resource: &Resource) { /* ... */ }
//! use_resource(&borrowed);
//! ```

#![warn(missing_docs, missing_debug_implementations)]
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "erasable")]
use erasable::{Erasable, ErasablePtr, ErasedPtr};
#[cfg(feature = "std")]
use std::{
    io,
    net::ToSocketAddrs,
    panic::{RefUnwindSafe, UnwindSafe},
};
use {
    alloc::{rc::Rc, sync::Arc},
    core::{
        borrow::Borrow,
        cmp::Ordering,
        fmt::{
            self, Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, Pointer, UpperExp,
            UpperHex,
        },
        hash::{Hash, Hasher},
        marker::PhantomData,
        mem::ManuallyDrop,
        ops::Deref,
        ptr,
    },
};

/// This trait is a polyfill for (`A`)`Rc::as_raw` and (`A`)`Rc::clone_raw`.
/// See https://internals.rust-lang.org/t/_/11463/11 for why these are important.
/// By using a trait here, we can more easily switch when these functions are available.
trait RawRc<T: ?Sized> {
    //noinspection RsSelfConvention
    fn as_raw(this: &Self) -> *const T;
    /// # Safety
    ///
    /// This pointer must have come from [`RawRc::as_raw`] or `into_raw`.
    unsafe fn clone_raw(this: *const T) -> Self;
}

impl<T: ?Sized> RawRc<T> for Arc<T> {
    #[rustfmt::skip]
    #[inline(always)]
    fn as_raw(this: &Self) -> *const T {
        #[cfg(not(has_Arc__as_raw))] {
            Arc::into_raw(unsafe { ptr::read(this) })
        }
        #[cfg(has_Arc__as_raw)] {
            Arc::as_raw(this)
        }
    }

    #[rustfmt::skip]
    #[inline(always)]
    unsafe fn clone_raw(this: *const T) -> Self {
        #[cfg(not(has_Arc__clone_raw))] {
            Arc::clone(&ManuallyDrop::new(Arc::from_raw(this)))
        }
        #[cfg(has_Arc__clone_raw)] {
            Arc::clone_raw(this)
        }
    }
}

impl<T: ?Sized> RawRc<T> for Rc<T> {
    #[rustfmt::skip]
    #[inline(always)]
    fn as_raw(this: &Self) -> *const T {
        #[cfg(not(has_Rc__as_raw))] {
            Rc::into_raw(unsafe { ptr::read(this) })
        }
        #[cfg(has_Rc__as_raw)] {
            Rc::as_raw(this)
        }
    }

    #[rustfmt::skip]
    #[inline(always)]
    unsafe fn clone_raw(this: *const T) -> Self {
        #[cfg(not(has_Rc__clone_raw))] {
            Rc::clone(&ManuallyDrop::new(Rc::from_raw(this)))
        }
        #[cfg(has_Rc__clone_raw)] {
            Rc::clone_raw(this)
        }
    }
}

// sigh, I almost got away without this...
macro_rules! doc_comment {
    ($doc:expr, $($tt:tt)*) => {
        #[doc = $doc]
        $($tt)*
    };
}

macro_rules! rc_borrow {
    ($($(#[$m:meta])* $vis:vis struct $RcBorrow:ident = &$Rc:ident;)*) => {$(
        $(#[$m])*
        $vis struct $RcBorrow<'a, T: ?Sized> {
            raw: ptr::NonNull<T>,
            marker: PhantomData<&'a $Rc<T>>
        }

        // NB: these cannot be `where &T: Send/Sync` as they allow upgrading to $Rc.
        unsafe impl<'a, T: ?Sized> Send for $RcBorrow<'a, T> where &'a $Rc<T>: Send {}
        unsafe impl<'a, T: ?Sized> Sync for $RcBorrow<'a, T> where &'a $Rc<T>: Sync {}

        impl<'a, T: ?Sized> From<&'a $Rc<T>> for $RcBorrow<'a, T> {
            fn from(v: &'a $Rc<T>) -> $RcBorrow<'a, T> {
                let raw = <$Rc<T> as RawRc<T>>::as_raw(v);
                $RcBorrow {
                    raw: unsafe { ptr::NonNull::new_unchecked(raw as *mut T) },
                    marker: PhantomData,
                }
            }
        }

        impl<'a, T: ?Sized> $RcBorrow<'a, T> {
            /// Convert this borrowed pointer into an owned pointer.
            $vis fn upgrade(this: Self) -> $Rc<T> {
                unsafe { <$Rc<T> as RawRc<T>>::clone_raw(this.raw.as_ptr()) }
            }

            /// Convert this borrowed pointer into a standard reference.
            ///
            /// This gives you a long-lived reference,
            /// whereas dereferencing gives a temporary borrow.
            $vis fn downgrade(this: Self) -> &'a T {
                unsafe { &*this.raw.as_ptr() }
            }

            /// Get a raw pointer that can be used with `from_raw`.
            $vis fn into_raw(this: Self) -> *const T {
                ManuallyDrop::new(this).raw.as_ptr()
            }

            doc_comment! {
                concat!("\
Construct a new `", stringify!($RcBorrow), "` from a raw pointer.

The raw pointer must have been previously returned by a call to
`",stringify!($RcBorrow),"<U>::into_raw` or `",stringify!($Rc),"<U>::as_raw`
where `U` must have the same size and alignment as `T`. This is trivially true
if `U` is `T`. Note that if `U` is not `T`, this is a pointer cast (transmute)
between the two types, and the types must be transmute-compatible."),
                $vis unsafe fn from_raw(ptr: *const T) -> Self {
                    $RcBorrow {
                        raw: ptr::NonNull::new_unchecked(ptr as *mut T),
                        marker: PhantomData
                    }
                }
            }
        }

        // ~~~ &T like impls ~~~ //

        #[cfg(feature = "erasable")]
        unsafe impl<T: ?Sized> ErasablePtr for $RcBorrow<'_, T>
        where
            T: Erasable
        {
            #[inline(always)]
            fn erase(this: Self) -> ErasedPtr {
                T::erase(this.raw)
            }

            #[inline(always)]
            unsafe fn unerase(this: ErasedPtr) -> Self {
                $RcBorrow {
                    raw: T::unerase(this),
                    marker: PhantomData,
                }
            }
        }

        impl<T: ?Sized, U: ?Sized> AsRef<U> for $RcBorrow<'_, T>
        where
            T: AsRef<U>,
        {
            fn as_ref(&self) -> &U {
                (**self).as_ref()
            }
        }

        impl<T: ?Sized> Binary for $RcBorrow<'_, T>
        where
            T: Binary,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> Borrow<T> for $RcBorrow<'_, T> {
            fn borrow(&self) -> &T {
                &**self
            }
        }

        impl<T: ?Sized> Clone for $RcBorrow<'_, T> {
            fn clone(&self) -> Self { *self }
        }

        // CoerceUnsized is unstable

        impl<T: ?Sized> Copy for $RcBorrow<'_, T> {}

        impl<T: ?Sized> Debug for $RcBorrow<'_, T>
        where
            T: Debug
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> Deref for $RcBorrow<'_, T> {
            type Target = T;
            fn deref(&self) -> &T {
                Self::downgrade(*self)
            }
        }

        // DispatchFromDyn is unstable

        impl<T: ?Sized> Display for $RcBorrow<'_, T>
        where
            T: Display,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> Eq for $RcBorrow<'_, T> where T: Eq {}

        // Fn, FnMut, FnOnce are unstable to implement

        impl<T: ?Sized> Hash for $RcBorrow<'_, T>
        where
            T: Hash,
        {
            fn hash<H: Hasher>(&self, state: &mut H) {
                (**self).hash(state)
            }
        }

        impl<T: ?Sized> LowerExp for $RcBorrow<'_, T>
        where
            T: LowerExp,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> LowerHex for $RcBorrow<'_, T>
        where
            T: LowerHex,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> Octal for $RcBorrow<'_, T>
        where
            T: Octal,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: Ord> Ord for $RcBorrow<'_, T>
        where
            T: Ord,
        {
            fn cmp(&self, other: &Self) -> Ordering {
                (**self).cmp(&**other)
            }
        }

        impl<T: ?Sized, O> PartialEq<O> for $RcBorrow<'_, T>
        where
            O: Deref,
            T: PartialEq<O::Target>,
        {
            fn eq(&self, other: &O) -> bool {
                (**self).eq(&*other)
            }
        }

        impl<T: ?Sized, O> PartialOrd<O> for $RcBorrow<'_, T>
        where
            O: Deref,
            T: PartialOrd<O::Target>,
        {
            fn partial_cmp(&self, other: &O) -> Option<Ordering> {
                (**self).partial_cmp(&*other)
            }
        }

        impl<T: ?Sized> Pointer for $RcBorrow<'_, T>
        where
            T: Pointer,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        #[cfg(feature = "std")]
        impl<T: ?Sized> ToSocketAddrs for $RcBorrow<'_, T>
        where
            T: ToSocketAddrs
        {
            type Iter = T::Iter;
            fn to_socket_addrs(&self) -> io::Result<T::Iter> {
                (**self).to_socket_addrs()
            }
        }

        impl<T: ?Sized> Unpin for $RcBorrow<'_, T> {}

        #[cfg(feature = "std")]
        impl<T: ?Sized> UnwindSafe for $RcBorrow<'_, T> where T: RefUnwindSafe {}

        impl<T: ?Sized> UpperExp for $RcBorrow<'_, T>
        where
            T: UpperExp,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> UpperHex for $RcBorrow<'_, T>
        where
            T: UpperHex,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }
    )*}
}

rc_borrow! {
    /// Borrowed version of [`Arc`].
    ///
    /// This type is guaranteed to have the same repr as `&T`.
    #[repr(transparent)]
    pub struct ArcBorrow = &Arc;
    /// Borrowed version of [`Rc`].
    ///
    /// This type is guaranteed to have the same repr as `&T`.
    #[repr(transparent)]
    pub struct RcBorrow = &Rc;
}
