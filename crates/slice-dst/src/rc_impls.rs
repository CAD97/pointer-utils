use {
    super::*,
    alloc::{rc::Rc, sync::Arc},
};

unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Rc<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        Box::new_slice_dst(len, init).into()
    }
}

unsafe impl<S: ?Sized + SliceDst> AllocSliceDst<S> for Arc<S> {
    unsafe fn new_slice_dst<I>(len: usize, init: I) -> Self
    where
        I: FnOnce(ptr::NonNull<S>),
    {
        Box::new_slice_dst(len, init).into()
    }
}
