#![deny(warnings)]
#![no_std]

extern crate alloc;
use slice_dst::SliceDst;

#[derive(SliceDst)]
#[repr(C)]
#[slice_dst(new_from_slice)]
struct Slice {
    my: u8,
    cool: u16,
    data: u32,
    tail: [u64],
}

impl Slice {
    // The derive implements a non-`pub` member with an awkward but fully general signature.
    // You will want to make a less awkward constructor that's targetted for your use case.
    pub fn new(my: u8, cool: u16, data: u32, tail: &[u64]) -> alloc::boxed::Box<Self> {
        Slice::new_from_slice((my, cool, data), tail)
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
}
