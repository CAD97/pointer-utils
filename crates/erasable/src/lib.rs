// NB: Last updated for Rust 1.40 parity. All impls are in rustdoc gutter order.
// Forwarding impls are provided for impls provided on Box.

//! Erase pointers of their concrete type and store type-erased pointers.
//!
//! This is roughly equivalent to C's `void*`, but it does not use `libc::c_void`.
//!
//! There are two main useful reasons to type erase pointers in Rust:
//!
//! - Removing viral generics from internal implementation details.
//!   If the internals truly don't care about the stored type,
//!   treating it opaquely reduces monomorphization cost
//!   both to the author and the compiler.
//! - Thin pointers to `?Sized` types. If an unsized type stores its metadata inline,
//!   then it can implement [`Erasable`] and be used behind type-erased pointers.
//!   The type erased pointer does not have to carry the metadata,
//!   and the fat pointer can be recovered from the inline metadata.
//!   We provide the [`Thin`] wrapper type to provide thin pointer types.

#![no_std]
#![cfg_attr(feature = "unstable_weak_into_raw", feature(weak_into_raw))]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, rc, sync};
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter, Pointer},
    future::Future,
    hash::{Hash, Hasher},
    iter::{FromIterator, FusedIterator},
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr,
    task::{Context, Poll},
};

/// A thin, type-erased pointer.
///
/// The `Erased` type is private, and should be treated as an opaque type.
/// When `extern type` is stabilized, `Erased` will be defined as one.
///
/// The current implementation uses a `struct Erased` with size 0 and align 1.
/// If you want to offset the pointer, make sure to cast to a `u8` or other known type pointer first.
/// When `Erased` becomes an extern type, it will properly have unknown size and align.
pub type ErasedPtr = ptr::NonNull<Erased>;

#[cfg(not(has_extern_type))]
pub(crate) use priv_in_pub::Erased;

#[cfg(not(has_extern_type))]
mod priv_in_pub {
    pub struct Erased; // extern type Erased
}

#[cfg(has_extern_type)]
extern "Rust" {
    pub type Erased;
}

/// A (smart) pointer type that can be type-erased (making a thin pointer).
///
/// When implementing this trait, you should implement it for all `Erasable` pointee types.
/// Implementing this trait allows use of the pointer in erased contexts, such as [`Thin`].
///
/// Additionally, implementors of this trait are required to have "smart pointer" `Deref` impls.
/// In other words, given an `ErasedPtr` erased from this type and a lifetime `'a` it is known
/// valid for, it must be valid to unerase the pointer, dereference it, and promote the produced
/// reference to `'a`. The produced reference must be into an allocation managed by this type,
/// not the stack representation of this type.
///
/// For example, `Box` implements `Erasable` because it's a pointer to a managed heap allocation.
/// [`Lazy`](https://docs.rs/once_cell/1.2.0/once_cell/sync/struct.Lazy.html), however,
/// `Deref`s into its own allocation, and as such, can not implement `Erasable`.
///
/// # Examples
///
/// ```rust
/// use erasable::*;
///
/// #[derive(Debug)]
/// struct MyBox<T: ?Sized>(Box<T>);
///
/// unsafe impl<T: ?Sized> ErasablePtr for MyBox<T>
/// where
///     T: Erasable
/// {
///     fn erase(this: Self) -> ErasedPtr {
///         ErasablePtr::erase(this.0)
///     }
///
///     unsafe fn unerase(this: ErasedPtr) -> Self {
///         Self(ErasablePtr::unerase(this))
///     }
/// }
///
/// let array = [0; 10];
/// let boxed = MyBox(Box::new(array));
/// let thin_box: Thin<MyBox<_>> = boxed.into();
/// dbg!(thin_box);
/// ```
pub unsafe trait ErasablePtr {
    /// Turn this erasable pointer into an erased pointer.
    ///
    /// To retrieve the original pointer, use `unerase`.
    fn erase(this: Self) -> ErasedPtr;

    /// Unerase this erased pointer.
    ///
    /// # Safety
    ///
    /// The erased pointer must have been created by `erase`.
    unsafe fn unerase(this: ErasedPtr) -> Self;
}

/// A pointee type that supports type-erased pointers (thin pointers).
///
/// This trait is automatically implemented for all sized types,
/// and can be manually implemented for unsized types that know their own metadata.
pub unsafe trait Erasable {
    /// Turn this erasable pointer into an erased pointer.
    ///
    /// To retrieve the original pointer, use `unerase`.
    fn erase(this: ptr::NonNull<Self>) -> ErasedPtr {
        erase(this)
    }

    /// Unerase this erased pointer.
    ///
    /// # Safety
    ///
    /// The erased pointer must have been created by `erase`.
    unsafe fn unerase(this: ErasedPtr) -> ptr::NonNull<Self>;
}

/// Erase a pointer.
pub fn erase<T: ?Sized>(ptr: ptr::NonNull<T>) -> ErasedPtr {
    unsafe { ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut Erased) }
}

/// Wrapper struct to create thin pointer types.
///
/// This type is guaranteed to have the same repr as [`ErasedPtr`].
///
/// # Examples
///
/// ```rust
/// use erasable::*;
///
/// let array = [0; 10];
/// let boxed = Box::new(array);
/// let thin_box: Thin<Box<_>> = boxed.into();
/// dbg!(thin_box);
/// ```
///
/// Note that this uses a `Sized` type: `[i32; 10]`.
/// This library does not provide erasable `?Sized` types.
#[repr(transparent)]
pub struct Thin<P: ErasablePtr> {
    ptr: ErasedPtr,
    marker: PhantomData<P>,
}

unsafe impl<P: ErasablePtr> Send for Thin<P> where P: Send {}
unsafe impl<P: ErasablePtr> Sync for Thin<P> where P: Sync {}

impl<P: ErasablePtr> From<P> for Thin<P> {
    fn from(this: P) -> Self {
        Thin {
            ptr: P::erase(this),
            marker: PhantomData,
        }
    }
}

impl<P: ErasablePtr> Thin<P> {
    fn inner(this: &Self) -> ManuallyDrop<P> {
        unsafe { ManuallyDrop::new(P::unerase(this.ptr)) }
    }

    // noinspection RsSelfConvention
    // `From` can't be impl'd because it's an impl on an uncovered type
    // `Into` can't be impl'd because it conflicts with the reflexive impl
    /// Extract the wrapped pointer.
    pub fn into_inner(this: Self) -> P {
        unsafe { P::unerase(ManuallyDrop::new(this).ptr) }
    }

    /// Run a closure with a borrow of the real pointer.
    pub fn with<F, T>(this: &Self, f: F) -> T
    where
        F: FnOnce(&P) -> T,
    {
        f(&Thin::inner(this))
    }

    /// Run a closure with a mutable borrow of the real pointer.
    pub fn with_mut<F, T>(this: &mut Self, f: F) -> T
    where
        F: FnOnce(&mut P) -> T,
    {
        f(&mut Thin::inner(this))
    }
}

impl<P: ErasablePtr> Drop for Thin<P> {
    fn drop(&mut self) {
        unsafe { P::unerase(self.ptr) };
    }
}

// ~~~ Box<T> like impls ~~~ //

impl<P: ErasablePtr, T: ?Sized> AsMut<T> for Thin<P>
where
    P: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        unsafe { Thin::with_mut(self, |p| erase_lt_mut(p.as_mut())) }
    }
}

impl<P: ErasablePtr, T: ?Sized> AsRef<T> for Thin<P>
where
    P: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        unsafe { Thin::with(self, |p| erase_lt(p.as_ref())) }
    }
}

// BorrowMut conflicts with reflexive impl

impl<P: ErasablePtr> Clone for Thin<P>
where
    P: Clone,
{
    fn clone(&self) -> Self {
        Thin::with(self, |this| this.clone()).into()
    }
}

// CoerceUnsized is unstable

impl<P: ErasablePtr> Debug for Thin<P>
where
    P: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Thin::with(self, |p| p.fmt(f))
    }
}

impl<P: ErasablePtr> Deref for Thin<P>
where
    P: Deref,
{
    type Target = P::Target;
    fn deref(&self) -> &P::Target {
        unsafe { Thin::with(self, |p| erase_lt(p)) }
    }
}

impl<P: ErasablePtr> DerefMut for Thin<P>
where
    P: DerefMut,
{
    fn deref_mut(&mut self) -> &mut P::Target {
        unsafe { Thin::with_mut(self, |p| erase_lt_mut(p)) }
    }
}

// DispatchFromDyn is unstable

impl<P: ErasablePtr> Display for Thin<P>
where
    P: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Thin::with(self, |p| p.fmt(f))
    }
}

impl<P: ErasablePtr> DoubleEndedIterator for Thin<P>
where
    P: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        Thin::with_mut(self, |p| p.next_back())
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        Thin::with_mut(self, |p| p.nth_back(n))
    }
}

impl<P: ErasablePtr> Eq for Thin<P> where P: Eq {}

impl<P: ErasablePtr> ExactSizeIterator for Thin<P> where P: ExactSizeIterator {}

// Fn, FnMut, FnOnce are unstable to implement

impl<P: ErasablePtr, A> FromIterator<A> for Thin<P>
where
    P: FromIterator<A>,
{
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        P::from_iter(iter).into()
    }
}

impl<P: ErasablePtr> FusedIterator for Thin<P> where P: FusedIterator {}

impl<P: ErasablePtr> Future for Thin<P>
where
    P: Future,
{
    type Output = P::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            Thin::with_mut(self.get_unchecked_mut(), |this| {
                Pin::new_unchecked(this).poll(cx)
            })
        }
    }
}

// Generator is unstable

impl<P: ErasablePtr> Hash for Thin<P>
where
    P: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        Thin::with(self, |p| p.hash(state))
    }
}

impl<P: ErasablePtr> Hasher for Thin<P>
where
    P: Hasher,
{
    fn finish(&self) -> u64 {
        Thin::with(self, |p| p.finish())
    }

    fn write(&mut self, bytes: &[u8]) {
        Thin::with_mut(self, |p| p.write(bytes))
    }

    fn write_u8(&mut self, i: u8) {
        Thin::with_mut(self, |p| p.write_u8(i))
    }

    fn write_u16(&mut self, i: u16) {
        Thin::with_mut(self, |p| p.write_u16(i))
    }

    fn write_u32(&mut self, i: u32) {
        Thin::with_mut(self, |p| p.write_u32(i))
    }

    fn write_u64(&mut self, i: u64) {
        Thin::with_mut(self, |p| p.write_u64(i))
    }

    fn write_u128(&mut self, i: u128) {
        Thin::with_mut(self, |p| p.write_u128(i))
    }

    fn write_usize(&mut self, i: usize) {
        Thin::with_mut(self, |p| p.write_usize(i))
    }

    fn write_i8(&mut self, i: i8) {
        Thin::with_mut(self, |p| p.write_i8(i))
    }

    fn write_i16(&mut self, i: i16) {
        Thin::with_mut(self, |p| p.write_i16(i))
    }

    fn write_i32(&mut self, i: i32) {
        Thin::with_mut(self, |p| p.write_i32(i))
    }

    fn write_i64(&mut self, i: i64) {
        Thin::with_mut(self, |p| p.write_i64(i))
    }

    fn write_i128(&mut self, i: i128) {
        Thin::with_mut(self, |p| p.write_i128(i))
    }

    fn write_isize(&mut self, i: isize) {
        Thin::with_mut(self, |p| p.write_isize(i))
    }
}

impl<P: ErasablePtr> Iterator for Thin<P>
where
    P: Iterator,
{
    type Item = P::Item;

    fn next(&mut self) -> Option<Self::Item> {
        Thin::with_mut(self, |p| p.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        Thin::with(self, |p| p.size_hint())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        Thin::with_mut(self, |p| p.nth(n))
    }
}

impl<P: ErasablePtr> Ord for Thin<P>
where
    P: Ord,
{
    fn cmp(&self, other: &Thin<P>) -> Ordering {
        Thin::with(self, |p| Thin::with(other, |other| p.cmp(other)))
    }
}

impl<P: ErasablePtr> PartialEq for Thin<P>
where
    P: PartialEq,
{
    fn eq(&self, other: &Thin<P>) -> bool {
        Thin::with(self, |p| Thin::with(other, |other| p.eq(other)))
    }
}

impl<P: ErasablePtr> PartialOrd for Thin<P>
where
    P: PartialOrd,
{
    fn partial_cmp(&self, other: &Thin<P>) -> Option<Ordering> {
        Thin::with(self, |p| Thin::with(other, |other| p.partial_cmp(other)))
    }
}

impl<P: ErasablePtr> Pointer for Thin<P>
where
    P: Pointer,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Thin::with(self, |p| p.fmt(f))
    }
}

// ~~~ impl Eraseable ~~~ //

unsafe impl<T> Erasable for T {
    unsafe fn unerase(this: ErasedPtr) -> ptr::NonNull<T> {
        this.cast()
    }
}

// ~~~ impl ErasablePtr ~~~ //

unsafe impl<T: ?Sized> ErasablePtr for ptr::NonNull<T>
where
    T: Erasable,
{
    fn erase(this: Self) -> ErasedPtr {
        T::erase(this)
    }

    unsafe fn unerase(this: ErasedPtr) -> Self {
        T::unerase(this)
    }
}

unsafe impl<T: ?Sized> ErasablePtr for &'_ T
where
    T: Erasable,
{
    fn erase(this: Self) -> ErasedPtr {
        T::erase(this.into())
    }

    unsafe fn unerase(this: ErasedPtr) -> Self {
        &*T::unerase(this).as_ptr()
    }
}

unsafe impl<T: ?Sized> ErasablePtr for &'_ mut T
where
    T: Erasable,
{
    fn erase(this: Self) -> ErasedPtr {
        T::erase(this.into())
    }

    unsafe fn unerase(this: ErasedPtr) -> Self {
        &mut *T::unerase(this).as_ptr()
    }
}

unsafe impl<P> ErasablePtr for Pin<P>
where
    P: ErasablePtr + Deref,
{
    fn erase(this: Self) -> ptr::NonNull<Erased> {
        unsafe { P::erase(Pin::into_inner_unchecked(this)) }
    }

    unsafe fn unerase(this: ptr::NonNull<Erased>) -> Self {
        Pin::new_unchecked(P::unerase(this))
    }
}

#[cfg(feature = "alloc")]
macro_rules! impl_erasable {
    (for<$T:ident> $($(#[$meta:meta])* $ty:ty),* $(,)?) => {$(
        $(#[$meta])*
        unsafe impl<$T: ?Sized> ErasablePtr for $ty
        where
            T: Erasable,
        {
            fn erase(this: Self) -> ErasedPtr {
                let ptr = unsafe { ptr::NonNull::new_unchecked(<$ty>::into_raw(this) as *mut _) };
                T::erase(ptr)
            }

            unsafe fn unerase(this: ErasedPtr) -> Self {
                Self::from_raw(T::unerase(this).as_ptr())
            }
        }
    )*}
}

#[cfg(feature = "alloc")]
impl_erasable!(for<T>
    Box<T>,
    sync::Arc<T>,
    #[cfg(feature = "unstable_weak_into_raw")]
    sync::Weak<T>,
    rc::Rc<T>,
    #[cfg(feature = "unstable_weak_into_raw")]
    rc::Weak<T>,
);

#[cfg(has_never)]
unsafe impl ErasablePtr for ! {
    fn erase(this: !) -> ErasedPtr {
        this
    }
    #[rustfmt::skip]
    unsafe fn unerase(_this: ErasedPtr) -> Self {
        #[cfg(debug_assertions)] {
            panic!("attempted to unerase erased pointer to !")
        }
        #[cfg(not(debug_assertions))] {
            core::hint::unreachable_unchecked()
        }
    }
}

unsafe fn erase_lt<'a, 'b, T: ?Sized>(this: &'a T) -> &'b T {
    &*(this as *const T)
}

unsafe fn erase_lt_mut<'a, 'b, T: ?Sized>(this: &'a mut T) -> &'b mut T {
    &mut *(this as *mut T)
}
