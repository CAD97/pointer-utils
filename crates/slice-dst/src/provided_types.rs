use super::*;

#[repr(C)]
#[derive(Debug, Eq, PartialEq, Hash)]
/// A custom slice-based DST.
///
/// The length is stored as a `usize` at offset 0.
/// This _must_ be the length of the trailing slice of the DST.
pub struct SliceWithHeader<Header, Item> {
    /// Safety: must be at offset 0
    length: usize,
    /// The included header. Does not dictate the slice length.
    pub header: Header,
    /// The included slice.
    pub slice: [Item],
}

unsafe impl<Header, Item> SliceDst for SliceWithHeader<Header, Item> {
    fn layout_for(len: usize) -> Layout {
        Self::layout(len).0
    }

    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self> {
        unsafe { ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
    }
}

impl<Header, Item> SliceWithHeader<Header, Item> {
    fn layout(len: usize) -> (Layout, [usize; 3]) {
        let length_layout = Layout::new::<usize>();
        let header_layout = Layout::new::<Header>();
        let slice_layout = Layout::array::<Item>(len).unwrap();
        polyfill::repr_c_3([length_layout, header_layout, slice_layout]).unwrap()
    }

    #[allow(clippy::new_ret_no_self)]
    /// Create a new slice/header DST in a [`AllocSliceDst`] container.
    ///
    /// # Panics
    ///
    /// Panics if the items iterator incorrectly reports its length.
    pub fn new<A, I>(header: Header, items: I) -> A
    where
        A: AllocSliceDst<Self>,
        I: IntoIterator<Item = Item>,
        I::IntoIter: ExactSizeIterator,
    {
        let items = items.into_iter();
        let len = items.len();

        struct InProgress<Header, Item> {
            raw: ptr::NonNull<SliceWithHeader<Header, Item>>,
            written: usize,
            layout: Layout,
            length_offset: usize,
            header_offset: usize,
            slice_offset: usize,
        }

        impl<Header, Item> Drop for InProgress<Header, Item> {
            fn drop(&mut self) {
                unsafe {
                    ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                        self.raw().add(self.slice_offset).cast::<Item>(),
                        self.written,
                    ));
                }
            }
        }

        impl<Header, Item> InProgress<Header, Item> {
            fn init(
                len: usize,
                header: Header,
                mut items: impl ExactSizeIterator<Item = Item>,
            ) -> impl FnOnce(ptr::NonNull<SliceWithHeader<Header, Item>>) {
                move |ptr| {
                    let mut this = Self::new(len, ptr);

                    unsafe {
                        for _ in 0..len {
                            let item = items
                                .next()
                                .expect("ExactSizeIterator over-reported length");
                            this.push(item);
                        }

                        assert!(
                            items.next().is_none(),
                            "ExactSizeIterator under-reported length"
                        );

                        this.finish(len, header)
                    }
                }
            }

            fn raw(&self) -> *mut u8 {
                self.raw.as_ptr().cast()
            }

            fn new(len: usize, raw: ptr::NonNull<SliceWithHeader<Header, Item>>) -> Self {
                let (layout, [length_offset, header_offset, slice_offset]) =
                    SliceWithHeader::<Header, Item>::layout(len);
                InProgress {
                    raw,
                    written: 0,
                    layout,
                    length_offset,
                    header_offset,
                    slice_offset,
                }
            }

            unsafe fn push(&mut self, item: Item) {
                self.raw()
                    .add(self.slice_offset)
                    .cast::<Item>()
                    .add(self.written)
                    .write(item);
                self.written += 1;
            }

            unsafe fn finish(self, len: usize, header: Header) {
                let this = ManuallyDrop::new(self);
                ptr::write(this.raw().add(this.length_offset).cast(), len);
                ptr::write(this.raw().add(this.header_offset).cast(), header);
                debug_assert_eq!(this.layout, Layout::for_value(this.raw.as_ref()))
            }
        }

        unsafe { A::new_slice_dst(len, InProgress::init(len, header, items)) }
    }

    #[allow(clippy::new_ret_no_self)]
    /// Create a new slice/header DST from a slice, in a [`AllocSliceDst`] container.
    pub fn from_slice<A>(header: Header, s: &[Item]) -> A
    where
        A: AllocSliceDst<Self>,
        Item: Copy,
    {
        let len = s.len();
        let (layout, [length_offset, header_offset, slice_offset]) = Self::layout(len);
        unsafe {
            A::new_slice_dst(len, |ptr| {
                let raw = ptr.as_ptr().cast::<u8>();
                ptr::write(raw.add(length_offset).cast(), len);
                ptr::write(raw.add(header_offset).cast(), header);
                ptr::copy_nonoverlapping(s.as_ptr(), raw.add(slice_offset).cast(), len);
                debug_assert_eq!(Layout::for_value(ptr.as_ref()), layout);
            })
        }
    }
}

impl<Header, Item> Clone for Box<SliceWithHeader<Header, Item>>
where
    Header: Clone,
    Item: Clone,
{
    fn clone(&self) -> Self {
        SliceWithHeader::new(self.header.clone(), self.slice.iter().cloned())
    }
}

#[cfg(feature = "erasable")]
unsafe impl<Header, Item> Erasable for SliceWithHeader<Header, Item> {
    unsafe fn unerase(this: ErasedPtr) -> ptr::NonNull<Self> {
        let len: usize = ptr::read(this.as_ptr().cast());
        let raw =
            ptr::NonNull::new_unchecked(ptr::slice_from_raw_parts_mut(this.as_ptr().cast(), len));
        Self::retype(raw)
    }

    const ACK_1_1_0: bool = true;
}

#[repr(C)]
#[derive(Debug, Eq, PartialEq, Hash)]
/// A custom str-based DST.
///
/// The length is stored as a `usize` at offset 0.
/// This _must_ be the length of the trailing slice.
pub struct StrWithHeader<Header> {
    /// Safety: must be at offset 0
    length: usize,
    /// The included header. Does not dictate the slice length.
    pub header: Header,
    /// The included str.
    pub str: str,
}

unsafe impl<Header> SliceDst for StrWithHeader<Header> {
    fn layout_for(len: usize) -> Layout {
        Self::layout(len).0
    }

    fn retype(ptr: ptr::NonNull<[()]>) -> ptr::NonNull<Self> {
        unsafe { ptr::NonNull::new_unchecked(ptr.as_ptr() as *mut _) }
    }
}

impl<Header> StrWithHeader<Header> {
    fn layout(len: usize) -> (Layout, [usize; 3]) {
        let length_layout = Layout::new::<usize>();
        let header_layout = Layout::new::<Header>();
        let slice_layout = Layout::array::<u8>(len).unwrap();
        polyfill::repr_c_3([length_layout, header_layout, slice_layout]).unwrap()
    }

    #[allow(clippy::new_ret_no_self)]
    /// Create a new str/header DST in a [`AllocSliceDst`] container.
    pub fn new<A>(header: Header, s: &str) -> A
    where
        A: AllocSliceDst<Self>,
    {
        let len = s.len();
        let (layout, [length_offset, header_offset, str_offset]) = Self::layout(len);
        unsafe {
            A::new_slice_dst(len, |ptr| {
                let raw = ptr.as_ptr().cast::<u8>();
                ptr::write(raw.add(length_offset).cast(), len);
                ptr::write(raw.add(header_offset).cast(), header);
                ptr::copy_nonoverlapping(s.as_bytes().as_ptr(), raw.add(str_offset).cast(), len);
                debug_assert_eq!(Layout::for_value(ptr.as_ref()), layout);
            })
        }
    }
}

impl<Header> Clone for Box<StrWithHeader<Header>>
where
    Header: Clone,
{
    fn clone(&self) -> Self {
        StrWithHeader::new(self.header.clone(), &self.str)
    }
}

#[cfg(feature = "erasable")]
unsafe impl<Header> Erasable for StrWithHeader<Header> {
    unsafe fn unerase(this: ErasedPtr) -> ptr::NonNull<Self> {
        let len: usize = ptr::read(this.as_ptr().cast());
        let raw =
            ptr::NonNull::new_unchecked(ptr::slice_from_raw_parts_mut(this.as_ptr().cast(), len));
        Self::retype(raw)
    }

    const ACK_1_1_0: bool = true;
}
