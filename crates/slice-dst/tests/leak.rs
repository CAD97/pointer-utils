use {
    slice_dst::*,
    std::{
        alloc::Layout,
        panic, ptr,
        rc::Rc,
        sync::{
            atomic::{AtomicUsize, Ordering::SeqCst},
            Arc,
        },
    },
};

struct DropTracking<'a> {
    place: &'a AtomicUsize,
}

impl<'a> DropTracking<'a> {
    fn new(place: &'a AtomicUsize) -> Self {
        place.fetch_add(1, SeqCst);
        DropTracking { place }
    }
}

impl Drop for DropTracking<'_> {
    fn drop(&mut self) {
        self.place.fetch_sub(1, SeqCst);
    }
}

#[test]
fn bad_exactsizeiterator() {
    struct Iter<'a> {
        counter: &'a AtomicUsize,
        len: usize,
    }

    impl ExactSizeIterator for Iter<'_> {
        fn len(&self) -> usize {
            self.len
        }
    }

    impl<'a> Iterator for Iter<'a> {
        type Item = DropTracking<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.len {
                0 | 1 => None,
                _ => {
                    self.len -= 1;
                    Some(DropTracking::new(self.counter))
                }
            }
        }
    }

    let mut counter = AtomicUsize::new(0);
    let _ = std::panic::catch_unwind(|| {
        let _: Box<_> = SliceWithHeader::new::<Box<_>, _>(
            DropTracking::new(&counter),
            Iter {
                counter: &counter,
                len: 5,
            },
        );
    });
    assert_eq!(*counter.get_mut(), 0);
}

#[allow(dead_code)]
struct S(u8);

unsafe impl SliceDst for S {
    fn layout_for(_: usize) -> Layout {
        Layout::new::<S>()
    }

    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self> {
        ptr.cast()
    }
}

#[test]
fn panic_in_init() {
    // This relies on miri to catch leaks
    let _ = std::panic::catch_unwind(|| {
        let _: Box<S> = unsafe { AllocSliceDst::new_slice_dst(0, |_| panic!()) };
    });
    let _ = std::panic::catch_unwind(|| {
        let _: Arc<S> = unsafe { AllocSliceDst::new_slice_dst(0, |_| panic!()) };
    });
    let _ = std::panic::catch_unwind(|| {
        let _: Rc<S> = unsafe { AllocSliceDst::new_slice_dst(0, |_| panic!()) };
    });
}
