#![deny(warnings)]
#![no_std]

extern crate alloc;
use slice_dst::SliceDst;

#[derive(SliceDst)]
#[repr(C)]
#[slice_dst(new_from_slice, new_from_iter)]
struct Slice {
    // NB: this struct is deliberately laid out in a noncompact way.
    // derive(SliceDst) relies on knowing the layout, which means repr(C).
    // Thus, you have to think about what order your data should be in.
    my: u8,
    cool: u16,
    data: u8,
    tail: [u32],
}

impl Slice {
    // The derive implements a non-`pub` member with an awkward but fully general signature.
    // You will want to make a less awkward constructor that's targetted for your use case.
    pub fn new(my: u8, cool: u16, data: u8, tail: &[u32]) -> alloc::boxed::Box<Self> {
        Slice::new_from_slice((my, cool, data), tail)
    }

    pub fn collect(
        my: u8,
        cool: u16,
        data: u8,
        tail: impl ExactSizeIterator + Iterator<Item = u32>,
    ) -> alloc::boxed::Box<Self> {
        Slice::new_from_iter((my, cool, data), tail)
    }
}

#[test]
fn it_works() {
    let slice = Slice::new(0, 1, 2, &[3, 4, 5, 6, 7]);
    assert!(matches!(
        *slice,
        Slice {
            my: 0,
            cool: 1,
            data: 2,
            tail: [3, 4, 5, 6, 7],
        }
    ));

    let slice = Slice::collect(0, 1, 2, 3..8);
    assert!(matches!(
        *slice,
        Slice {
            my: 0,
            cool: 1,
            data: 2,
            tail: [3, 4, 5, 6, 7],
        }
    ));
}
