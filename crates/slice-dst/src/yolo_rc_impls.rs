use {
    super::*,
    alloc::{rc::Rc, sync::Arc},
    core::{cell::UnsafeCell, mem},
};

#[repr(C)]
#[allow(dead_code)]
struct RcHeapLayout<T: ?Sized> {
    strong: UnsafeCell<usize>,
    weak: UnsafeCell<usize>,
    value: T,
}

unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Rc<S> {
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
        Rc::from_raw(ptr.as_ptr())
    }
}

unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Arc<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        let rc: Rc<S> = Rc::new_slice_dst(len, init);
        mem::transmute(rc)
    }
}
