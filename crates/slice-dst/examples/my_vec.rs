use std::{
    fmt::Display,
    mem::{self, MaybeUninit},
    ops::Deref,
};

use ref_cast::RefCast;
use slice_dst::SliceWithHeader;

/// Default capacity of [`MyVec`].
const MY_VEC_DEFAULT_CAPACITY: usize = 4;

/// On the heap we will store the number of used elements in the slice (length)
/// and a slice of (maybe uninitialized) values.
type HeapData<T> = SliceWithHeader<usize, MaybeUninit<T>>;

/// Our [`Vec`] implementation.
///
/// _Note:_ In contrast to [`std::vec::Vec`] this stores its length on the heap.
struct MyVec<T>(Box<HeapData<T>>);

impl<T> MyVec<T> {
    /// Empty [`MyVec`] with [default capacity](MY_VEC_DEFAULT_CAPACITY).
    fn new() -> Self {
        let inner = SliceWithHeader::new(
            0,
            (0..MY_VEC_DEFAULT_CAPACITY).map(|_| MaybeUninit::uninit()),
        );
        Self(inner)
    }
    /// Double the capacity of [`MyVec`].
    ///
    /// Initialized elements are copied to the new allocated slice.
    fn grow(&mut self) {
        // Create an `ExactSizeIterator` double the size as the previous capacity.
        let iter = (0..2 * self.capacity()).map(|_| MaybeUninit::uninit());
        // Allocate a new DST.
        let new = Self(SliceWithHeader::new(self.0.header, iter));
        let mut old = mem::replace(self, new);
        for idx in 0..old.0.header {
            // Swap old, initialized values with new, uninitialized ones.
            mem::swap(&mut self.0.slice[idx], &mut old.0.slice[idx])
        }
        // Reset length to prevent drop of uninitialized values.
        old.0.header = 0;
    }
    fn push(&mut self, element: T) {
        if self.len() == self.capacity() {
            self.grow();
        }
        let len = &mut self.0.header;
        self.0.slice[*len] = MaybeUninit::new(element);
        *len += 1;
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        let len = self.len();
        self.0.slice.iter_mut().take(len).for_each(|t| {
            unsafe {
                // Safe as only initialized values iterated.
                std::ptr::drop_in_place(mem::transmute::<_, *mut T>(t));
            };
        })
    }
}

impl<T> Deref for MyVec<T> {
    type Target = MySlice<T>;

    fn deref(&self) -> &Self::Target {
        MySlice::ref_cast(&self.0)
    }
}

impl<T> AsRef<MySlice<T>> for MyVec<T> {
    fn as_ref(&self) -> &MySlice<T> {
        &*self
    }
}

/// The slice we get from a [`MyVec`].
///
/// We use the `ref-cast` crate to wrap the [`HeapData`] in our new-type
/// which allows us to implement our own functions.
#[derive(RefCast)]
#[repr(transparent)]
struct MySlice<T>(HeapData<T>);

impl<T> MySlice<T> {
    fn len(&self) -> usize {
        self.0.header
    }
    fn capacity(&self) -> usize {
        self.0.slice.len()
    }
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.slice.iter().take(self.len()).map(|t| unsafe {
            // Safe as only the initialized elements are iterated.
            mem::transmute(t)
        })
    }
}

/// As [`MyVec`] implements [`Deref`] we can pass in a `&MyVec`.
fn print_my_vec<T: Display>(slice: &MySlice<T>) {
    for (idx, t) in slice.iter().enumerate() {
        println!("{}. element: {}", idx, t);
    }
}

fn main() {
    let mut my_vec = MyVec::new();
    assert_eq!(MY_VEC_DEFAULT_CAPACITY, my_vec.capacity());
    assert_eq!(0, my_vec.len());

    my_vec.push("one");
    my_vec.push("two");
    my_vec.push("three");
    my_vec.push("four");
    my_vec.push("five");
    assert_eq!(2 * MY_VEC_DEFAULT_CAPACITY, my_vec.capacity());
    assert_eq!(5, my_vec.len());
    print_my_vec(&my_vec);
}
