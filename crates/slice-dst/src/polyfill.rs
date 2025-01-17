// #![allow(deprecated)] // this is a polyfill module

use core::alloc::{Layout, LayoutError};

#[inline]
pub(crate) fn pad_layout_to_align(this: &Layout) -> Layout {
    let pad = layout_padding_needed_for(this, this.align());
    let new_size = this.size() + pad;
    unsafe { Layout::from_size_align_unchecked(new_size, this.align()) }
}

#[inline]
pub(crate) fn repr_c_3(fields: [Layout; 3]) -> Result<(Layout, [usize; 3]), LayoutError> {
    let mut offsets: [usize; 3] = [0; 3];
    let mut layout = fields[0];
    for i in 1..3 {
        let (new_layout, this_offset) = layout.extend(fields[i])?;
        layout = new_layout;
        offsets[i] = this_offset;
    }
    Ok((pad_layout_to_align(&layout), offsets))
}

#[inline]
fn layout_padding_needed_for(this: &Layout, align: usize) -> usize {
    let len = this.size();
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}

pub(crate) fn ptr_dangling_at<T>(addr: usize) -> *mut T {
    #[cfg(not(has_strict_provenance))]
    {
        addr as _
    }
    #[cfg(has_strict_provenance)]
    {
        core::ptr::without_provenance_mut(addr)
    }
}
