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

#[cfg(has_ptr_slice_from_raw_parts)]
use core::ptr::slice_from_raw_parts_mut as slice_from_raw_parts;
#[cfg(not(has_ptr_slice_from_raw_parts))]
use core::slice::from_raw_parts_mut as slice_from_raw_parts;
#[cfg(feature = "erasable")]
use erasable::{Erasable, ErasedPtr};
use {
    alloc::{
        alloc::{alloc, dealloc, handle_alloc_error},
        boxed::Box,
    },
    core::{alloc::Layout, mem::ManuallyDrop, ptr, slice},
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

unsafe impl<T> SliceDst for [T] {
    fn layout_for(len: usize) -> Layout {
        layout_polyfill::layout_array::<T>(len).unwrap()
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
            ptr::NonNull::new(layout.align() as *mut ())
        } else {
            ptr::NonNull::new(alloc(layout) as *mut ())
        }
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
        init(ptr.0);
        ptr.finalize()
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

    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self> {
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
        let items = items.into_iter();
        let len = items.len();

        struct InProgress<Header, Item> {
            raw: ptr::NonNull<SliceWithHeader<Header, Item>>,
            written: usize,
            layout: Layout,
            length_offset: usize,
            header_offset: usize,
            slice_offset: usize,
        }

        impl<Header, Item> Drop for InProgress<Header, Item> {
            fn drop(&mut self) {
                unsafe {
                    ptr::drop_in_place(slice_from_raw_parts(
                        self.raw().add(self.slice_offset).cast::<Item>(),
                        self.written,
                    ));
                }
            }
        }

        impl<Header, Item> InProgress<Header, Item> {
            fn init(
                len: usize,
                header: Header,
                mut items: impl Iterator<Item = Item> + ExactSizeIterator,
            ) -> impl FnOnce(ptr::NonNull<SliceWithHeader<Header, Item>>) {
                move |ptr| {
                    let mut this = Self::new(len, ptr);

                    unsafe {
                        for _ in 0..len {
                            let item = items
                                .next()
                                .expect("ExactSizeIterator over-reported length");
                            this.push(item);
                        }

                        assert!(
                            items.next().is_none(),
                            "ExactSizeIterator under-reported length"
                        );

                        this.finish(len, header)
                    }
                }
            }

            fn raw(&self) -> *mut u8 {
                self.raw.as_ptr().cast()
            }

            fn new(len: usize, raw: ptr::NonNull<SliceWithHeader<Header, Item>>) -> Self {
                let (layout, [length_offset, header_offset, slice_offset]) =
                    SliceWithHeader::<Header, Item>::layout(len);
                InProgress {
                    raw,
                    written: 0,
                    layout,
                    length_offset,
                    header_offset,
                    slice_offset,
                }
            }

            unsafe fn push(&mut self, item: Item) {
                self.raw()
                    .add(self.slice_offset)
                    .cast::<Item>()
                    .add(self.written)
                    .write(item);
                self.written += 1;
            }

            unsafe fn finish(self, len: usize, header: Header) {
                let this = ManuallyDrop::new(self);
                ptr::write(this.raw().add(this.length_offset).cast(), len);
                ptr::write(this.raw().add(this.header_offset).cast(), header);
                debug_assert_eq!(this.layout, Layout::for_value(this.raw.as_ref()))
            }
        }

        unsafe { A::new_slice_dst(len, InProgress::init(len, header, items)) }
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
        let len: usize = ptr::read(this.as_ptr().cast());
        let raw = ptr::NonNull::new_unchecked(slice_from_raw_parts(this.as_ptr().cast(), len));
        Self::retype(raw)
    }

    const ACK_1_1_0: bool = true;
}

pub(crate) mod layout_polyfill;

#[cfg(yolo_rc_layout_known)]
mod yolo_rc_impls;

#[cfg(not(yolo_rc_layout_known))]
mod rc_impls;
