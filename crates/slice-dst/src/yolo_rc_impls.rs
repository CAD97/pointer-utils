//! The heap layout of both `Rc` and `Arc` are two `usize` unsafe cells and the data payload.
//! The value has to be at the end of the heap, because it could be an unsized trailing type.
//! This is not guaranteed by the standard library, and is just an implementation detail.
//! Nonetheless, we abuse that when `YOLO_RC_HEAP_LAYOUT_KNOWN` is set to allocate directly.
//! Otherwise, custom DSTs must be allocated in a `Box` and then moved (the impl without env var).

use {
    super::*,
    alloc::{rc::Rc, sync::Arc},
    core::{cell::UnsafeCell, mem},
};

#[repr(C)]
struct RcHeapLayout<T: ?Sized> {
    strong: UnsafeCell<usize>,
    weak: UnsafeCell<usize>,
    value: T,
}

unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<RcHeapLayout<T>> for Box<RcHeapLayout<T>> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        let mut value_offset = None;
        let ptr: ptr::NonNull<S> = alloc_slice_dst_in(
            |layout| {
                let (layout, offset) = layout_polyfill::extend_layout(
                    &layout_polyfill::extend_layout(
                        &Layout::new::<usize>(),
                        Layout::new::<usize>(),
                    )
                    .unwrap()
                    .0,
                    layout,
                )
                .unwrap();
                value_offset = Some(offset);
                layout_polyfill::pad_layout_to_align(&layout)
            },
            len,
        );
        let value_offset = value_offset.unwrap();
        let raw = ptr.as_ptr() as *mut usize;
        ptr::write(raw.offset(0), 1);
        ptr::write(raw.offset(1), 1);
        init(S::retype(
            slice::from_raw_parts_mut(ptr.cast::<u8>().as_ptr().add(value_offset).cast(), len)
                .into(),
        ));
        Box::from_raw(ptr.as_ptr())
    }
}

//noinspection DuplicatedCode
unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Rc<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        let boxed: Box<RcHeapLayout<S>> = Box::<RcHeapLayout<S>>::new_slice_dst(len, init);
        mem::transmute(boxed)
    }
}

//noinspection DuplicatedCode
unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Arc<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        let boxed: Box<RcHeapLayout<S>> = Box::<RcHeapLayout<S>>::new_slice_dst(len, init);
        mem::transmute(boxed)
    }
}
