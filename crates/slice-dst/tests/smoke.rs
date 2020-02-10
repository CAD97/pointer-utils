//! These tests don't really assert anything, they just exercise the API.
//! This is primarily intended to be run under miri as a sanitizer.

#![allow(unused, clippy::redundant_clone)]

use {erasable::Thin, slice_dst::*, std::sync::Arc};

#[test]
fn slice() {
    let slice: Vec<u32> = vec![0, 1, 2, 3, 4, 5];
    let slice: Box<SliceWithHeader<(), u32>> = SliceWithHeader::new((), slice);
    assert_eq!(slice.slice, [0, 1, 2, 3, 4, 5]);
    let slice = slice.clone();
}

#[test]
fn zst() {
    let slice: Vec<()> = vec![(); 16];
    let slice: Box<SliceWithHeader<(), ()>> = SliceWithHeader::new((), slice);
    let slice = slice.clone();
}

type Data = usize;
#[repr(transparent)]
#[derive(Debug, Clone)]
struct Node(Thin<Arc<SliceWithHeader<Data, Node>>>);

// NB: the wrapper type is required, as the type alias version
//     type Node = Thin<Arc<SliceWithHeader<Data, Node>>>
// is rejected as an infinitely recursive type alias expansion.

impl Node {
    fn new<I>(head: Data, children: I) -> Self
    where
        I: IntoIterator<Item = Node>,
        I::IntoIter: ExactSizeIterator,
    {
        Node(SliceWithHeader::new::<Arc<_>, I>(head, children).into())
    }

    fn data(&self) -> usize {
        self.0.header
    }
}

#[test]
fn node() {
    let a = Node::new(1, vec![]);
    let b = Node::new(2, vec![]);
    let c = Node::new(3, vec![]);
    let children = vec![a.clone(), b.clone(), c.clone()];
    let boxed = Node::new(children.iter().map(|node| node.data()).sum(), children);
    assert_eq!(boxed.data(), 6);
    dbg!(boxed);
}
