#![deny(warnings)]

use slice_dst::SliceDst;

#[derive(SliceDst)]
#[repr(C)]
struct Slice {
    my: u32,
    cool: u32,
    data: u32,
    tail: [u32],
}
