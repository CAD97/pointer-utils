#![warn(missing_docs, missing_debug_implementations)]
#![no_std]

//! Support for custom slice-based DSTs.
//!
//! By handling allocation manually, we can manually allocate the `Box` for a custom DST.
//! So long as the size lines up with what it should be, once the metadata is created,
//! Rust actually already handles the DSTs it already supports perfectly well, safely!
//! Setting them up is the hard part, which this crate handles for you.
//!
//! # Examples
//!
//! We have a tree structure! Each node holds some data and its children array.
//! In normal Rust, you would probably typically implement it something like this:
//!
//! ```rust
//! # use std::sync::Arc;
//! struct Node {
//!     data: &'static str,
//!     children: Vec<Arc<Node>>,
//! }
//!
//! let a = Node { data: "a", children: vec![] };
//! let b = Node { data: "b", children: vec![] };
//! let c = Node { data: "c", children: vec![] };
//! let abc = Node { data: "abc", children: vec![a.into(), b.into(), c.into()] };
//! ```
//!
//! With this setup, the memory layout looks vaguely like the following diagram:
//!
//! ```text
//!                                              +--------------+
//!                                              |Node          |
//!                                        +---->|data: "a"     |
//! +------------+    +---------------+    |     |children: none|
//! |Node        |    |Vec<Arc<Node>> |    |     +--------------+
//! |data: "abc" |    |[0]: +--------------+     |Node          |
//! |children: +----->|[1]: +------------------->|data: "b"     |
//! +------------+    |[2]: +--------------+     |children: none|
//!                   +---------------|    |     +--------------+
//!                                        |     |Node          |
//!                                        +---->|data: "c"     |
//!                                              |children: none|
//!                                              +--------------+
//! ```
//!
//! With this crate, however, the children array can be stored inline with the node's data:
//!
//! ```rust
//! # use std::{iter, sync::Arc}; use slice_dst::*;
//! struct Node(Arc<SliceWithHeader<&'static str, Node>>);
//!
//! let a = Node(SliceWithHeader::new("a", None));
//! let b = Node(SliceWithHeader::new("b", None));
//! let c = Node(SliceWithHeader::new("c", None));
//! // this vec is just an easy way to get an ExactSizeIterator
//! let abc = Node(SliceWithHeader::new("abc", vec![a, b, c]));
//! ```
//!
//! ```text
//!                          +-----------+
//! +-------------+          |Node       |
//! |Node         |    +---->|length: 0  |
//! |length: 3    |    |     |header: "a"|
//! |header: "abc"|    |     +-----------+
//! |slice: [0]: +-----+     |Node       |
//! |       [1]: +---------->|length: 0  |
//! |       [2]: +-----+     |header: "b"|
//! +-------------+    |     +-----------+
//!                    |     |Node       |
//!                    +---->|length: 0  |
//!                          |header: "c"|
//!                          +------------
//! ```
//!
//! The exact times you will want to use this rather than just standard types varries.
//! This is mostly useful when space optimization is very important.
//! This is still useful when using an arena: it reduces the allocations in the arena
//! in exchange for moving node payloads to the heap alongside the children array.
//!
//! # But how?
//!
//! This is possible because of the following key building blocks:
//!
//! - `Box`'s [memory layout][boxed-memory-layout] is defined and uses the
//!   [global allocator][std::alloc::Global], and is allowed to be manually allocated.
//! - [Array layout][array-layout] and [slice layout][slice-layout] are defined.
//! - [`#[repr(C)]`][repr-c-layout] allows us to make compound types with defined layout.
//! - We can turn an opaque pointer into a slice fat pointer with
//!   [`ptr::slice_from_raw_parts`].
//! - We can cast a slice pointer to a pointer to our compound type
//!   in order to keep the correct fat pointer metadata.
//!
//! So with these guarantees, we can "just" manually allocate some space, initialize it
//! for some custom `repr(C)` structure, and convert it into a `Box`. From that point,
//! `Box` handles managing the memory, including deallocation or moving it into another
//! smart pointer, such as `Arc`.
//!
//!   [boxed-memory-layout]: <https://doc.rust-lang.org/stable/std/boxed/index.html#memory-layout>
//!   [array-layout]: <https://doc.rust-lang.org/stable/reference/type-layout.html#array-layout>
//!   [slice-layout]: <https://doc.rust-lang.org/stable/reference/type-layout.html#slice-layout>
//!   [repr-c-layout]: <https://doc.rust-lang.org/stable/reference/type-layout.html#reprc-structs>
//!   [std::alloc::Global]: <https://doc.rust-lang.org/stable/std/alloc/index.html#the-global_allocator-attribute>
//!
//! [`SliceDst`] defines the capabilities required of the pointee type. It must be able to
//! turn a trailing slice length into a [`Layout`] for the whole pointee, and it must provide
//! a way to turn a untyped slice pointer `*mut [()]` into a correctly typed pointer.
//!
//! The functions [`alloc_slice_dst`] and [`alloc_slice_dst_in`] provide a way
//! to allocate space for a `SliceDst` type via the global allocator.
//!
//! [`AllocSliceDst`] types are owning heap pointers that can create a new slice DST.
//! They take an initialization routine that is responsible for initializing the
//! uninitialized allocated place, and do the ceremony required to allocate the place
//! and turn it into the proper type by delgating to `SliceDst` and `alloc_slice_dst`.
//! They also handle panic/unwind safety of the initialization routine and prevent
//! leaking of the allocated place due to an initialization panic.
//!
//! [`TryAllocSliceDst`] is the potentially fallible initialization version.
//!
//! All of these pieces are the glue, but [`SliceWithHeader`] and [`StrWithHeader`]
//! put the pieces together into a safe package. They take a header and an iterator
//! (or copyable slice) and put together all of the pieces to allocate a dynamically
//! sized custom type.
//!
//! Additionaly, though not strictly required, these types store the slice length inline.
//! This gives them the ability to reconstruct pointers from fully type erased pointers
#![cfg_attr(feature = "erasable", doc = "via the [`Erasable`] trait")]
//! .

// All hail Chairity!
// The one who saves our sanity -
// blessing us with Clarity.
// Queen of popularity.
// When haboo becomes a rarity -
// we thank Yoba for Chairity.
// https://twitch.tv/thehaboo

extern crate alloc;

#[cfg(feature = "erasable")]
use erasable::{Erasable, ErasedPtr};
use {
    alloc::{
        alloc::{alloc, dealloc, handle_alloc_error},
        boxed::Box,
        rc::Rc,
        sync::Arc,
    },
    core::{alloc::Layout, mem::ManuallyDrop, ptr},
};

/// A custom slice-based dynamically sized type.
///
/// Unless you are making a custom slice DST that needs to pack its length extremely well,
/// then you should just use [`SliceWithHeader`] instead.
///
/// # Safety
///
/// Must be implemented as described and may be relied upon by generic code.
pub unsafe trait SliceDst {
    /// Get the layout of the slice-containing type with the given slice length.
    fn layout_for(len: usize) -> Layout;

    /// Add the type onto an untyped pointer.
    ///
    /// This is used to add the type on during allocation.
    /// This function is required because otherwise Rust cannot
    /// guarantee that the metadata on both sides of the cast lines up.
    ///
    /// # Safety
    ///
    /// The implementation _must not_ dereference the input pointer.
    /// This function is safe because it must work for all input pointers,
    /// without asserting the pointer's validity of any kind, express or implied,
    /// including but not limited to the validities of alignment, fitness for
    /// dereferencing and nullity.
    ///
    /// In practice, this means that the implementation should just be a pointer cast.
    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self>;
}

unsafe impl<T> SliceDst for [T] {
    fn layout_for(len: usize) -> Layout {
        Layout::array::<T>(len).unwrap()
    }

    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self> {
        unsafe { ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
    }
}

/// Allocate a slice-based DST with the [global allocator][`alloc()`].
///
/// The returned pointer is owned and completely uninitialized;
/// you are required to initialize it correctly.
///
/// If the type to be allocated has zero size,
/// then an arbitrary aligned dangling nonnull pointer is returned.
pub fn alloc_slice_dst<S: ?Sized + SliceDst>(len: usize) -> ptr::NonNull<S> {
    alloc_slice_dst_in(|it| it, len)
}

/// Allocate a slice-based DST with the [global allocator][`alloc()`] within some container.
///
/// The returned pointer is owned and completely uninitialized;
/// you are required to initialize it correctly.
///
/// Note that while this function returns a `ptr::NonNull<S>`,
/// the pointer is to the allocation as specified by `container(S::layout(len))`,
/// so if you want/need a pointer to `S`, you will need to offset it.
///
/// If the layout to be allocated has zero size,
/// then an arbitrary aligned dangling nonnull pointer is returned.
pub fn alloc_slice_dst_in<S: ?Sized + SliceDst, F>(container: F, len: usize) -> ptr::NonNull<S>
where
    F: FnOnce(Layout) -> Layout,
{
    let layout = container(S::layout_for(len));
    unsafe {
        let ptr = if layout.size() == 0 {
            // Do not allocate in the ZST case! CAD97/pointer-utils#23
            ptr::NonNull::new(polyfill::ptr_dangling_at(layout.align()))
        } else {
            ptr::NonNull::new(alloc(layout) as *mut ())
        }
        .unwrap_or_else(|| handle_alloc_error(layout));
        let ptr = ptr::NonNull::new_unchecked(ptr::slice_from_raw_parts_mut(ptr.as_ptr(), len));
        S::retype(ptr)
    }
}

/// Types that can allocate a custom slice DST within them.
///
/// # Implementation note
///
/// For most types, [`TryAllocSliceDst`] should be the implementation primitive.
/// This trait can then be implemented in terms of `TryAllocSliceDst`:
///
/// ```rust
/// # use {slice_dst::*, std::ptr};
/// # struct Container<T: ?Sized>(Box<T>);
/// # unsafe impl<S: ?Sized + SliceDst> TryAllocSliceDst<S> for Container<S> {
/// #     unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
/// #     where I: FnOnce(ptr::NonNull<S>) -> Result<(), E>
/// #     { unimplemented!() }
/// # }
/// unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Container<S> {
///     unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
///     where
///         I: FnOnce(ptr::NonNull<S>),
///     {
///         enum Void {} // or never (!) once it is stable
///         #[allow(clippy::unit_arg)]
///         let init = |ptr| Ok::<(), Void>(init(ptr));
///         match Self::try_new_slice_dst(len, init) {
///             Ok(a) => a,
///             Err(void) => match void {},
///         }
///     }
/// }
/// ```
///
/// This is not a blanket impl due to coherence rules; if the blanket impl were present,
/// it would be impossible to implement `AllocSliceDst` instead of `TryAllocSliceDst`.
///
/// # Safety
///
/// Must be implemented as described and may be relied upon by generic code.
pub unsafe trait AllocSliceDst<S: ?Sized + SliceDst> {
    /// Create a new custom slice DST.
    ///
    /// # Safety
    ///
    /// `init` must properly initialize the object behind the pointer.
    /// `init` receives a fully uninitialized pointer and must not read anything before writing.
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>);
}

// FUTURE: export? Would need better generic support.
macro_rules! impl_alloc_by_try_alloc {
    ($T:ident) => {
        unsafe impl<S: ?Sized + SliceDst> $crate::AllocSliceDst<S> for $T<S> {
            unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
            where
                I: FnOnce(::core::ptr::NonNull<S>),
            {
                enum Void {}
                #[allow(clippy::unit_arg)]
                let init = |ptr| ::core::result::Result::<(), Void>::Ok(init(ptr));
                match <Self as $crate::TryAllocSliceDst<S>>::try_new_slice_dst(len, init) {
                    Ok(a) => a,
                    Err(void) => match void {},
                }
            }
        }
    };
}

/// Types that can allocate a custom slice DST within them,
/// given a fallible initialization function.
///
/// # Safety
///
/// Must be implemented as described and may be relied upon by generic code.
pub unsafe trait TryAllocSliceDst<S: ?Sized + SliceDst>: AllocSliceDst<S> + Sized {
    /// Create a new custom slice DST with a fallible initialization function.
    ///
    /// # Safety
    ///
    /// `init` must properly initialize the object behind the pointer.
    /// `init` receives a fully uninitialized pointer and must not read anything before writing.
    ///
    /// If the initialization closure panics or returns an error,
    /// the allocated place will be deallocated but not dropped.
    /// To clean up the partially initialized type, we suggest
    /// proxying creation through scope guarding types.
    unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
    where
        I: FnOnce(ptr::NonNull<S>) -> Result<(), E>;
}

// SAFETY: Box is guaranteed to be allocatable by GlobalAlloc.
impl_alloc_by_try_alloc!(Box);
unsafe impl<S: ?Sized + SliceDst> TryAllocSliceDst<S> for Box<S> {
    unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
    where
        I: FnOnce(ptr::NonNull<S>) -> Result<(), E>,
    {
        struct RawBox<S: ?Sized + SliceDst>(ptr::NonNull<S>, Layout);

        impl<S: ?Sized + SliceDst> RawBox<S> {
            unsafe fn new(len: usize) -> Self {
                let layout = S::layout_for(len);
                RawBox(alloc_slice_dst(len), layout)
            }

            unsafe fn finalize(self) -> Box<S> {
                let this = ManuallyDrop::new(self);
                Box::from_raw(this.0.as_ptr())
            }
        }

        impl<S: ?Sized + SliceDst> Drop for RawBox<S> {
            fn drop(&mut self) {
                unsafe {
                    dealloc(self.0.as_ptr().cast(), self.1);
                }
            }
        }

        let ptr = RawBox::new(len);
        init(ptr.0)?;
        Ok(ptr.finalize())
    }
}

// SAFETY: just delegates to `Box`'s implementation (for now?)
impl_alloc_by_try_alloc!(Rc);
unsafe impl<S: ?Sized + SliceDst> TryAllocSliceDst<S> for Rc<S> {
    unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
    where
        I: FnOnce(ptr::NonNull<S>) -> Result<(), E>,
    {
        Box::try_new_slice_dst(len, init).map(Into::into)
    }
}

// SAFETY: just delegates to `Box`'s implementation (for now?)
impl_alloc_by_try_alloc!(Arc);
unsafe impl<S: ?Sized + SliceDst> TryAllocSliceDst<S> for Arc<S> {
    unsafe fn try_new_slice_dst<I, E>(len: usize, init: I) -> Result<Self, E>
    where
        I: FnOnce(ptr::NonNull<S>) -> Result<(), E>,
    {
        Box::try_new_slice_dst(len, init).map(Into::into)
    }
}

pub(crate) mod polyfill;
mod provided_types;

pub use provided_types::{SliceWithHeader, StrWithHeader};
