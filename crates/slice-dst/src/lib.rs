#![no_std]

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

pub fn alloc_slice_dst<S: ?Sized + SliceDst>(len: usize) -> ptr::NonNull<S> {
    alloc_slice_dst_in(|it| it, len)
}

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

pub unsafe trait AllocSliceDst<S: ?Sized + SliceDst> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>);
}

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
pub struct SliceWithHeader<Header, Item> {
    /// Safety: must be at offset 0
    length: usize,
    pub header: Header,
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
