use {
    slice_dst::*,
    std::{
        panic,
        sync::atomic::{AtomicUsize, Ordering::SeqCst},
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
#[cfg_attr(
    all(miri, target_os = "windows"),
    ignore = "miri does not support panicking on windows rust-lang/miri#1059"
)]
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
