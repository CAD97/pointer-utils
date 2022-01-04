// NB: Last updated for Rust 1.40 parity. All impls are in rustdoc gutter order.

//! Known unique versions of [`Rc`] and [`Arc`].
//! This allows them to be used for mutable ownership.
//!
//! The main reason to use [`RcBox`] or [`ArcBox`] is for types that will be reference counted,
//! but need some "fixing up" done after being allocated behind the reference counted pointer.
//! With the standard library types, you would use `get_mut` and have to handle the impossible
//! case where the value was shared. With the known unique versions, you have [`DerefMut`],
//! so it's as simple as mutating behind a [`Box`].

#![warn(missing_docs, missing_debug_implementations)]
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "erasable")]
use erasable::{Erasable, ErasablePtr, ErasedPtr};
#[cfg(feature = "slice-dst")]
use slice_dst::{AllocSliceDst, SliceDst, TryAllocSliceDst};
#[cfg(feature = "std")]
use std::panic::UnwindSafe;
use {
    alloc::{boxed::Box, rc::Rc, string::String, sync::Arc, vec::Vec},
    core::{
        any::Any,
        borrow::{Borrow, BorrowMut},
        cmp::Ordering,
        convert::{TryFrom, TryInto},
        fmt::{self, Debug, Display, Formatter, Pointer},
        hash::{Hash, Hasher},
        hint::unreachable_unchecked,
        iter::{FromIterator, FusedIterator},
        marker::PhantomData,
        mem::ManuallyDrop,
        ops::{Deref, DerefMut},
        pin::Pin,
        ptr,
    },
};

macro_rules! doc_comment {
    ($x:expr, $($tt:tt)*) => {
        #[doc = $x]
        $($tt)*
    };
}

macro_rules! rc_box {
    ($($(#[$m:meta])* $RcBox:ident = $Rc:ident)*) => {$(
        $(#[$m])*
        pub struct $RcBox<T: ?Sized> {
            raw: ptr::NonNull<T>,
            marker: PhantomData<$Rc<T>>,
        }

        unsafe impl<T: ?Sized> Send for $RcBox<T> where Box<T>: Send {}
        unsafe impl<T: ?Sized> Sync for $RcBox<T> where Box<T>: Sync {}

        impl<T: ?Sized> Drop for $RcBox<T> {
            fn drop(&mut self) {
                unsafe { drop($Rc::<T>::from(ptr::read(self))) }
            }
        }

        impl<T: ?Sized> From<$RcBox<T>> for $Rc<T> {
            fn from(v: $RcBox<T>) -> $Rc<T> {
                unsafe { $Rc::from_raw($RcBox::into_raw(v).as_ptr()) }
            }
        }

        impl<T: ?Sized> TryFrom<$Rc<T>> for $RcBox<T> {
            type Error = $Rc<T>;
            fn try_from(mut v: $Rc<T>) -> Result<$RcBox<T>, $Rc<T>> {
                // Could this just be `$Rc::strong_count == 1 && $Rc::weak_count == 0`?
                // I _think_ `get_mut` has the weaker synchronization requirements?
                if $Rc::get_mut(&mut v).is_some() {
                    unsafe { Ok($RcBox::from_raw($Rc::into_raw(v))) }
                } else {
                    Err(v)
                }
            }
        }

        impl<T: ?Sized> TryFrom<Pin<$Rc<T>>> for Pin<$RcBox<T>> {
            type Error = Pin<$Rc<T>>;
            fn try_from(v: Pin<$Rc<T>>) -> Result<Pin<$RcBox<T>>, Pin<$Rc<T>>> {
                unsafe {
                    let v = Pin::into_inner_unchecked(v);
                    match $RcBox::<T>::try_from(v) {
                        Ok(this) => Ok(Pin::new_unchecked(this)),
                        Err(v) => Err(Pin::new_unchecked(v)),
                    }
                }
            }
        }

        impl<T: ?Sized> $RcBox<T> {
            unsafe fn from_unchecked<V>(v: V) -> Self
            where
                V: TryInto<$RcBox<T>>,
            {
                v.try_into().unwrap_or_else(|_| unreachable_unchecked())
            }
        }

        // ~~~ $Rc<T> and Box<T> like inherent impls ~~~ //

        impl $RcBox<dyn Any + 'static> {
            doc_comment! {
                concat!("Attempt to downcast the box to a concrete type.

# Examples

```rust
# use rc_box::*; use std::convert::TryInto;
# use std::rc::Rc; use std::sync::Arc;
use std::any::Any;

fn print_if_string(value: ", stringify!($RcBox), r#"<dyn Any>) {
    if let Ok(string) = value.downcast::<String>() {
        println!("String ({}): {}", string.len(), string);
    }
}

let my_string = "Hello World".to_string();
let my_string: "#, stringify!($Rc), "<dyn Any> = ", stringify!($Rc), "::new(my_string); 
print_if_string(my_string.try_into().unwrap());
let my_number: ", stringify!($Rc), "<dyn Any> = ", stringify!($Rc), "::new(0i8);
print_if_string(my_number.try_into().unwrap());
```

The unsizing as `", stringify!($Rc), "` is required until
[DST coercions](https://github.com/rust-lang/rust/issues/27732) are stabilized. Alternatively,
activate the `unsize` feature to convert the pointer via an explicit method call."),
                #[inline]
                pub fn downcast<T>(self) -> Result<$RcBox<T>, Self>
                where T: Any,
                {
                    if self.is::<T>() {
                        unsafe {
                            let raw: *mut dyn Any = Self::into_raw(self).as_ptr();
                            Ok($RcBox::from_raw(raw as *mut T))
                        }
                    } else {
                        Err(self)
                    }
                }
            }
        }

        impl $RcBox<dyn Any + 'static + Send> {
        doc_comment! {
                concat!("Attempt to downcast the box to a concrete type.

# Examples

```rust
# use rc_box::*; use std::convert::TryInto;
# use std::rc::Rc; use std::sync::Arc;
use std::any::Any;

fn print_if_string(value: ", stringify!($RcBox), r#"<dyn Any + Send>) {
    if let Ok(string) = value.downcast::<String>() {
        println!("String ({}): {}", string.len(), string);
    }
}

let my_string = "Hello World".to_string();
let my_string: "#, stringify!($Rc), "<dyn Any + Send> = ", stringify!($Rc), "::new(my_string); 
print_if_string(my_string.try_into().unwrap());
let my_number: ", stringify!($Rc), "<dyn Any + Send> = ", stringify!($Rc), "::new(0i8);
print_if_string(my_number.try_into().unwrap());
```

The unsizing as `", stringify!($Rc), "` is required until
[DST coercions](https://github.com/rust-lang/rust/issues/27732) are stabilized. Alternatively,
activate the `unsize` feature to convert the pointer via an explicit method call."),
                #[inline]
                pub fn downcast<T>(self) -> Result<$RcBox<T>, Self>
                where T: Any + Send
                {
                    if self.is::<T>() {
                        unsafe {
                            let raw: *mut (dyn Any + Send) = Self::into_raw(self).as_ptr();
                            Ok($RcBox::from_raw(raw as *mut T))
                        }
                    } else {
                        Err(self)
                    }
                }
            }
        }

        impl $RcBox<dyn Any + 'static + Send + Sync> {
        doc_comment! {
                concat!("Attempt to downcast the box to a concrete type.

# Examples

```rust
# use rc_box::*; use std::convert::TryInto;
# use std::rc::Rc; use std::sync::Arc;
use std::any::Any;

fn print_if_string(value: ", stringify!($RcBox), r#"<dyn Any + Send + Sync>) {
    if let Ok(string) = value.downcast::<String>() {
        println!("String ({}): {}", string.len(), string);
    }
}

let my_string = "Hello World".to_string();
let my_string: "#, stringify!($Rc), "<dyn Any + Send + Sync> = ", stringify!($Rc), "::new(my_string); 
print_if_string(my_string.try_into().unwrap());
let my_number: ", stringify!($Rc), "<dyn Any + Send + Sync> = ", stringify!($Rc), "::new(0i8);
print_if_string(my_number.try_into().unwrap());
```

The unsizing as `", stringify!($Rc), "` is required until
[DST coercions](https://github.com/rust-lang/rust/issues/27732) are stabilized. Alternatively,
activate the `unsize` feature to convert the pointer via an explicit method call."),
                #[inline]
                pub fn downcast<T>(self) -> Result<$RcBox<T>, Self>
                where T: Any + Send + Sync
                {
                    if self.is::<T>() {
                        unsafe {
                            let raw: *mut (dyn Any + Send + Sync) = Self::into_raw(self).as_ptr();
                            Ok($RcBox::from_raw(raw as *mut T))
                        }
                    } else {
                        Err(self)
                    }
                }
            }
        }

        impl<T: ?Sized> $RcBox<T> {
            // `downgrade` makes no sense as it would always immediately drop.

            doc_comment! {
                concat!("Construct an ", stringify!($RcBox), " from a raw pointer.

# Safety

The raw pointer must have previously been acquired by a call to [`",
stringify!($RcBox), "::into_raw`], or [`", stringify!($Rc), "::into_raw`]
where the `", stringify!($Rc), "` is known unique."),
                pub unsafe fn from_raw(ptr: *const T) -> Self {
                    $RcBox {
                        // NB: $Rc::from_raw uses `ptr::NonNull::new_unchecked`
                        raw: ptr::NonNull::new_unchecked(ptr as *mut _),
                        marker: PhantomData,
                    }
                }
            }

            doc_comment! {
                concat!("Get a mutable reference into the `", stringify!($RcBox), "`.

This method exists only for API compatibility with `", stringify!($Rc), "`.
Use `DerefMut` instead."),
                #[deprecated(note = "Use [`DerefMut`](#impl-DerefMut) instead")]
                pub fn get_mut(this: &mut Self) -> Option<&mut T> {
                    Some(&mut **this)
                }
            }

            doc_comment! {
                concat!("Get a mutable reference into the `", stringify!($RcBox), "`.

This method exists only for API compatibility with `", stringify!($Rc), "`.
Use `DerefMut` instead."),
                #[deprecated(note = "Use [`DerefMut`](#impl-DerefMut) instead")]
                pub fn get_mut_unchecked(this: &mut Self) -> &mut T {
                    &mut **this
                }
            }

            doc_comment! {
                concat!("\
Returns a raw pointer to the object `T` pointed to by this `", stringify!($RcBox), "`.

Note that this returns a [`ptr::NonNull`], not a raw pointer.
That makes this function equivalent to `as_raw_non_null`."),
                pub fn as_raw(this: &Self) -> ptr::NonNull<T> {
                    this.raw
                }
            }

            doc_comment! {
                concat!("\
Consume the `", stringify!($RcBox), "`, returning the wrapped pointer.

To avoid a memory leak, the pointer must be converted back to a `",
stringify!($RcBox), "`, using [`", stringify!($RcBox), "::from_raw`],
or directly into a `", stringify!($Rc), "`, using [`", stringify!($Rc), "::from_raw`].

Note that this returns a [`ptr::NonNull`], not a raw pointer.
That makes this function equivalent to `into_raw_non_null`."),
                pub fn into_raw(this: Self) -> ptr::NonNull<T> {
                    $RcBox::as_raw(&ManuallyDrop::new(this))
                }
            }

            doc_comment! {
                concat!("Consume and leak the `", stringify!($RcBox), "`."),
                pub fn leak<'a>(this: Self) -> &'a mut T
                where
                    T: 'a,
                {
                    unsafe { &mut *$RcBox::into_raw(this).as_ptr() }
                }
            }

            doc_comment! {
                concat!("Create a new ", stringify!($RcBox), "."),
                pub fn new(data: T) -> Self
                where
                    T: Sized,
                {
                    unsafe { $RcBox::from_unchecked($Rc::new(data)) }
                }
            }

            // `new_uninit`/`new_uninit_slice` are unstable but probably desirable.

            doc_comment! {
                concat!("\
Construct a new `Pin<", stringify!($RcBox), "<T>>`. If `T` does not implement [`Unpin`],
then the data will be pinned in memory and unable to be moved."),
                pub fn pin(x: T) -> Pin<$RcBox<T>>
                where
                    T: Sized,
                {
                    unsafe {
                        Pin::new_unchecked($RcBox::from_unchecked(
                            Pin::into_inner_unchecked($Rc::pin(x))
                        ))
                    }
                }
            }

            doc_comment! {
                concat!("Deconstruct this `", stringify!($RcBox), "`, returning the inner value."),
                pub fn into_inner(this: Self) -> T
                where
                    T: Sized,
                {
                    let rc: $Rc<T> = this.into();
                    $Rc::try_unwrap(rc).unwrap_or_else(|_| unsafe { unreachable_unchecked() })
                }
            }
        }

        // ~~~ Box<T> like impls ~~~ //

        #[cfg(feature = "erasable")]
        unsafe impl<T: ?Sized> ErasablePtr for $RcBox<T>
        where
            T: Erasable
        {
            fn erase(this: Self) -> ErasedPtr {
                T::erase($RcBox::into_raw(this))
            }

            unsafe fn unerase(this: ErasedPtr) -> Self {
                $RcBox::from_raw(T::unerase(this).as_ptr())
            }
        }

        #[cfg(feature = "slice-dst")]
        unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for $RcBox<S> {
            unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
            where
                I: FnOnce(ptr::NonNull<S>),
            {
                Self::from_unchecked($Rc::new_slice_dst(len, init))
            }
        }
        #[cfg(feature = "slice-dst")]
        unsafe impl<S: ?Sized + SliceDst> TryAllocSliceDst<S> for $RcBox<S> {
            unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
            where
                I: FnOnce(ptr::NonNull<S>) -> Result<(), E>,
            {
                $Rc::try_new_slice_dst(len, init).map(|x| Self::from_unchecked(x))
            }
        }

        impl<T: ?Sized> AsMut<T> for $RcBox<T> {
            fn as_mut(&mut self) -> &mut T {
                &mut **self
            }
        }

        impl<T: ?Sized> AsRef<T> for $RcBox<T> {
            fn as_ref(&self) -> &T {
                &**self
            }
        }

        impl<T: ?Sized> Borrow<T> for $RcBox<T> {
            fn borrow(&self) -> &T {
                &**self
            }
        }

        impl<T: ?Sized> BorrowMut<T> for $RcBox<T> {
            fn borrow_mut(&mut self) -> &mut T {
                &mut **self
            }
        }

        // impl CoerceUnsized

        impl<T: ?Sized> Debug for $RcBox<T>
        where
            T: Debug,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> Deref for $RcBox<T> {
            type Target = T;
            fn deref(&self) -> &T {
                unsafe { self.raw.as_ref() }
            }
        }

        impl<T: ?Sized> DerefMut for $RcBox<T> {
            fn deref_mut(&mut self) -> &mut T {
                unsafe { self.raw.as_mut() }
            }
        }

        // impl DispatchFromDyn

        impl<T: ?Sized> Display for $RcBox<T>
        where
            T: Display,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (**self).fmt(f)
            }
        }

        impl<T: ?Sized> DoubleEndedIterator for $RcBox<T>
        where
            T: DoubleEndedIterator,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                (**self).next_back()
            }

            fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
                (**self).nth_back(n)
            }
        }

        impl<T: ?Sized> Eq for $RcBox<T> where T: Eq {}

        impl<T: ?Sized> ExactSizeIterator for $RcBox<T> where T: ExactSizeIterator {}

        // impl Fn, FnMut, FnOnce

        impl<T> From<&'_ [T]> for $RcBox<[T]>
        where
            T: Clone
        {
            fn from(v: &[T]) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl From<&'_ str> for $RcBox<str> {
            fn from(v: &str) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl<T: ?Sized> From<Box<T>> for $RcBox<T> {
            fn from(v: Box<T>) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl From<String> for $RcBox<str> {
            fn from(v: String) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl<T> From<T> for $RcBox<T> {
            fn from(v: T) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl<T> From<Vec<T>> for $RcBox<[T]> {
            fn from(v: Vec<T>) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from(v)) }
            }
        }

        impl<T> FromIterator<T> for $RcBox<[T]> {
            fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                unsafe { $RcBox::from_unchecked($Rc::from_iter(iter)) }
            }
        }

        impl<T: ?Sized> FusedIterator for $RcBox<T> where T: FusedIterator {}

        // Skip Future/Generator; just use Box instead! There's no reason to share it later.

        impl<T: ?Sized> Hash for $RcBox<T>
        where
            T: Hash,
        {
            fn hash<H: Hasher>(&self, state: &mut H) {
                (**self).hash(state)
            }
        }

        impl<T: ?Sized> Hasher for $RcBox<T>
        where
            T: Hasher,
        {
            fn finish(&self) -> u64 {
                (**self).finish()
            }

            fn write(&mut self, bytes: &[u8]) {
                (**self).write(bytes)
            }

            fn write_u8(&mut self, i: u8) {
                (**self).write_u8(i)
            }

            fn write_u16(&mut self, i: u16) {
                (**self).write_u16(i)
            }

            fn write_u32(&mut self, i: u32) {
                (**self).write_u32(i)
            }

            fn write_u64(&mut self, i: u64) {
                (**self).write_u64(i)
            }

            fn write_u128(&mut self, i: u128) {
                (**self).write_u128(i)
            }

            fn write_usize(&mut self, i: usize) {
                (**self).write_usize(i)
            }

            fn write_i8(&mut self, i: i8) {
                (**self).write_i8(i)
            }

            fn write_i16(&mut self, i: i16) {
                (**self).write_i16(i)
            }

            fn write_i32(&mut self, i: i32) {
                (**self).write_i32(i)
            }

            fn write_i64(&mut self, i: i64) {
                (**self).write_i64(i)
            }

            fn write_i128(&mut self, i: i128) {
                (**self).write_i128(i)
            }

            fn write_isize(&mut self, i: isize) {
                (**self).write_isize(i)
            }
        }

        impl<T: ?Sized> Iterator for $RcBox<T>
        where
            T: Iterator
        {
            type Item = T::Item;

            fn next(&mut self) -> Option<Self::Item> {
                (**self).next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (**self).size_hint()
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                (**self).nth(n)
            }
        }

        impl<T: ?Sized> Ord for $RcBox<T>
        where
            T: Ord,
        {
            fn cmp(&self, other: &Self) -> Ordering {
                (**self).cmp(other)
            }
        }

        impl<T: ?Sized, O> PartialEq<O> for $RcBox<T>
        where
            O: Deref,
            T: PartialEq<O::Target>,
        {
            fn eq(&self, other: &O) -> bool {
                (**self).eq(&*other)
            }
        }

        impl<T: ?Sized, O> PartialOrd<O> for $RcBox<T>
        where
            O: Deref,
            T: PartialOrd<O::Target>,
        {
            fn partial_cmp(&self, other: &O) -> Option<Ordering> {
                (**self).partial_cmp(other)
            }
        }

        impl<T: ?Sized> Pointer for $RcBox<T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                fmt::Pointer::fmt(&&**self, f)
            }
        }

        // impl TryFrom<($Rc)(Box)<[T]>> for $RcBox<[T; N]>
        // (waiting on const generics)

        impl<T: ?Sized> Unpin for $RcBox<T> {}

        #[cfg(feature = "std")]
        impl<T: ?Sized> UnwindSafe for $RcBox<T> where Box<T>: UnwindSafe {}

        #[cfg(feature = "unsize")]
        doc_comment! {
            concat!("Unsizes a pointer using the `unsize` crate.

# Usage

```
# use rc_box::*;
use unsize::{Coercion, CoerceUnsize};

let unique = ", stringify!($RcBox), "::new(|| 42u32);
let unique:", stringify!($RcBox), r"<dyn Fn() -> u32> =
    unique.unsize(Coercion::<_, dyn Fn() -> u32>::to_fn());

let value = (*unique)();
assert_eq!(value, 42);
```

Another common usage would be to create a `dyn Any`.

fn print_if_string(value: ", stringify!($RcBox), r#"<dyn Any>) {
    if let Ok(string) = value.downcast::<String>() {
        println!("String ({}): {}", string.len(), string);
    }
}

let my_string = "Hello World".to_string();
let my_string: "#, stringify!($RcBox), "<dyn Any> = ", stringify!($RcBox), "::new(my_string).unsize(Coercion::to_any());
print_if_string(my_string);
let my_number: ", stringify!($RcBox), "<dyn Any> = ", stringify!($RcBox), "::new(0i8).unsize(Coercion::to_any());
print_if_string(my_number);
```"),
            unsafe impl<T, U: ?Sized> unsize::CoerciblePtr<U> for $RcBox<T> {
                type Pointee = T;
                type Output = $RcBox<U>;
                fn as_sized_ptr(&mut self) -> *mut T {
                    $RcBox::as_raw(self).as_ptr()
                }
                unsafe fn replace_ptr(self, new: *mut U) -> $RcBox<U> {
                    let new = $RcBox::into_raw(self).replace_ptr(new);
                    $RcBox::from_raw(new.as_ptr() as *const U)
                }
            }
        }
    )*};
}

rc_box! {
    /// Known unique version of [`Arc`].
    ///
    /// This type is guaranteed to have the same repr as `Box<T>`.
    /// (The heap layout is that of `Arc<T>`.)
    #[repr(transparent)]
    ArcBox = Arc
    /// Known unique version of [`Rc`].
    ///
    /// This type is guaranteed to have the same repr as `Box<T>`.
    /// (The heap layout is that of `Rc<T>`.)
    #[repr(transparent)]
    RcBox = Rc
}
