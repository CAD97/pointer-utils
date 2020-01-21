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

// All hail Chairity!
// The one who saves our sanity -
// blessing us with Clarity.
// Queen of popularity.
// When haboo becomes a rarity -
// we thank Yoba for Chairity.
// https://twitch.tv/thehaboo

extern crate alloc;

use core::ptr::NonNull;
#[cfg(feature = "erasable")]
use erasable::{Erasable, ErasedPtr};
use {
    alloc::{
        alloc::{alloc, handle_alloc_error},
        boxed::Box,
    },
    core::{alloc::Layout, ptr, slice},
};

/// A custom slice-based dynamically sized type.
///
/// Unless you are making a custom slice DST that needs to pack its length extremely well,
/// then you should just use [`SliceWithHeader`] instead.
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

/// Allocate a slice-based DST with the [global allocator][`alloc()`].
///
/// The returned pointer is owned and completely uninitialized;
/// you are required to initialize it correctly.
pub fn alloc_slice_dst<S: ?Sized + SliceDst>(len: usize) -> ptr::NonNull<S> {
    alloc_slice_dst_in(|it| it, len)
}

/// Allocate a slice-based DST with the [global allocator][`alloc()`] within some container.
///
/// The returned pointer is owned and completely uninitialized;
/// you are required to initialize it correctly.
pub fn alloc_slice_dst_in<S: ?Sized + SliceDst, F>(container: F, len: usize) -> ptr::NonNull<S>
where
    F: FnOnce(Layout) -> Layout,
{
    let layout = container(S::layout_for(len));
    unsafe {
        let ptr = ptr::NonNull::new(alloc(layout) as *mut ())
            .unwrap_or_else(|| handle_alloc_error(layout));
        let ptr = ptr::NonNull::new_unchecked(slice::from_raw_parts_mut::<()>(ptr.as_ptr(), len));
        S::retype(ptr)
    }
}

/// Types that can allocate a custom slice DST within them.
pub unsafe trait AllocSliceDst<S: ?Sized + SliceDst> {
    /// Create a new custom slice DST.
    ///
    /// # Safety
    ///
    /// `init` must properly initialize the object behind the pointer.
    /// The stored length of the slice DST must be the same as the length used in this call.
    /// `init` receives a fully uninitialized pointer and must not read anything before writing.
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>);
}

// SAFETY: Box is guaranteed to be allocatable by GlobalAlloc.
unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Box<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        let ptr = alloc_slice_dst(len);
        init(ptr);
        Box::from_raw(ptr.as_ptr())
    }
}

#[repr(C)]
#[derive(Debug, Eq, PartialEq, Hash)]
/// A custom slice-based DST.
///
/// The length is stored as a `usize` at offset 0.
/// This _must_ be the length of the trailing slice of the DST.
pub struct SliceWithHeader<Header, Item> {
    /// Safety: must be at offset 0
    length: usize,
    /// The included header. Does not dictate the slice length.
    pub header: Header,
    /// The included slice.
    pub slice: [Item],
}

unsafe impl<Header, Item> SliceDst for SliceWithHeader<Header, Item> {
    fn layout_for(len: usize) -> Layout {
        Self::layout(len).0
    }

    fn retype(ptr: NonNull<[()]>) -> NonNull<Self> {
        unsafe { ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
    }
}

impl<Header, Item> SliceWithHeader<Header, Item> {
    fn layout(len: usize) -> (Layout, [usize; 3]) {
        let length_layout = Layout::new::<usize>();
        let header_layout = Layout::new::<Header>();
        let slice_layout = layout_polyfill::layout_array::<Item>(len).unwrap();
        layout_polyfill::repr_c_3([length_layout, header_layout, slice_layout]).unwrap()
    }

    #[allow(clippy::new_ret_no_self)]
    /// Create a new slice/header DST in a [`AllocSliceDst`] container.
    ///
    /// # Panics
    ///
    /// Panics if the items iterator incorrectly reports its length.
    pub fn new<A, I>(header: Header, items: I) -> A
    where
        A: AllocSliceDst<Self>,
        I: IntoIterator<Item = Item>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut items = items.into_iter();
        let len = items.len();
        let (layout, [length_offset, header_offset, slice_offset]) = Self::layout(len);

        unsafe {
            A::new_slice_dst(len, |ptr| {
                let raw = ptr.as_ptr().cast::<u8>();
                ptr::write(raw.add(length_offset).cast(), len);
                ptr::write(raw.add(header_offset).cast(), header);
                let mut slice_ptr = raw.add(slice_offset).cast::<Item>();
                for _ in 0..len {
                    let item = items
                        .next()
                        .expect("ExactSizeIterator over-reported length");
                    ptr::write(slice_ptr, item);
                    slice_ptr = slice_ptr.offset(1);
                }
                assert!(
                    items.next().is_none(),
                    "ExactSizeIterator under-reported length"
                );
                assert_eq!(layout, Layout::for_value(ptr.as_ref()));
            })
        }
    }
}

impl<Header, Item> Clone for Box<SliceWithHeader<Header, Item>>
where
    Header: Clone,
    Item: Clone,
{
    fn clone(&self) -> Self {
        SliceWithHeader::new(self.header.clone(), self.slice.iter().cloned())
    }
}

#[cfg(feature = "erasable")]
unsafe impl<Header, Item> Erasable for SliceWithHeader<Header, Item> {
    unsafe fn unerase(this: ErasedPtr) -> ptr::NonNull<Self> {
        #[cfg(not(has_ptr_slice_from_raw_parts))]
        let slice_from_raw_parts = slice::from_raw_parts_mut::<()>;
        #[cfg(has_ptr_slice_from_raw_parts)]
        let slice_from_raw_parts = ptr::slice_from_raw_parts_mut::<()>;

        let len: usize = ptr::read(this.as_ptr().cast());
        let raw = ptr::NonNull::new_unchecked(slice_from_raw_parts(this.as_ptr().cast(), len));
        Self::retype(raw)
    }
}

pub(crate) mod layout_polyfill;

#[cfg(yolo_rc_layout_known)]
mod yolo_rc_impls;

#[cfg(not(yolo_rc_layout_known))]
mod rc_impls;
