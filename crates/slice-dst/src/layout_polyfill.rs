#![allow(deprecated)] // this is a polyfill module

use core::{
    alloc::{Layout, LayoutErr},
    cmp,
};

#[inline]
pub(crate) fn extend_layout(this: &Layout, next: Layout) -> Result<(Layout, usize), LayoutErr> {
    let new_align = cmp::max(this.align(), next.align());
    let pad = layout_padding_needed_for(&this, next.align());
    let offset = this.size().checked_add(pad).ok_or_else(layout_err)?;
    let new_size = offset.checked_add(next.size()).ok_or_else(layout_err)?;
    let layout = Layout::from_size_align(new_size, new_align)?;
    Ok((layout, offset))
}

#[inline]
pub(crate) fn pad_layout_to_align(this: &Layout) -> Layout {
    let pad = layout_padding_needed_for(this, this.align());
    let new_size = this.size() + pad;
    unsafe { Layout::from_size_align_unchecked(new_size, this.align()) }
}

#[inline]
pub(crate) fn layout_array<T>(n: usize) -> Result<Layout, LayoutErr> {
    repeat_layout(&Layout::new::<T>(), n).map(|(k, _)| k)
}

#[inline]
pub(crate) fn repr_c_3(fields: [Layout; 3]) -> Result<(Layout, [usize; 3]), LayoutErr> {
    let mut offsets: [usize; 3] = [0; 3];
    let mut layout = fields[0];
    for i in 1..3 {
        let (new_layout, this_offset) = extend_layout(&layout, fields[i])?;
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

#[inline]
fn repeat_layout(this: &Layout, n: usize) -> Result<(Layout, usize), LayoutErr> {
    let padded_size = pad_layout_to_align(this).size();
    let alloc_size = padded_size.checked_mul(n).ok_or_else(layout_err)?;
    unsafe {
        Ok((
            Layout::from_size_align_unchecked(alloc_size, this.align()),
            padded_size,
        ))
    }
}

#[inline]
fn layout_err() -> LayoutErr {
    Layout::from_size_align(0, 0).unwrap_err()
}
