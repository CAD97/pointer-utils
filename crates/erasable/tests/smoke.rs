//! These tests don't really assert anything, they just exercise the API.
//! This is primarily intended to be run under miri as a sanitizer.

#![allow(unused, clippy::redundant_clone, clippy::unnecessary_operation)]

use erasable::{Erasable, ErasablePtr, ErasedPtr, Thin};

#[derive(Copy, Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
struct Big([u128; 32]);

#[test]
fn erasing() {
    let boxed: Box<Big> = Box::new(Big::default());
    let ptr = &*boxed as *const _ as usize;
    let erased: ErasedPtr = ErasablePtr::erase(boxed);
    assert_eq!(erased.as_ptr() as usize, ptr);
    let boxed: Box<Big> = unsafe { ErasablePtr::unerase(erased) };
    assert_eq!(&*boxed as *const _ as usize, ptr);
}

#[test]
fn thinning() {
    let boxed: Box<Big> = Default::default();
    let mut thin: Thin<Box<Big>> = boxed.into();
    let thin_ref: Thin<&Big> = (&*thin).into();
    *thin_ref;
    // Unfortunately, because Thin: Drop necessarily, NLL lifetime shortening doesn't work.
    // Because of this, you shouldn't really work with Thin<&_> as a stack variable.
    // Especially since it needs to be converted back into a fat reference to be used at all.
    // Thin<P> is a type for storage, not for transient stack existence.
    drop(thin_ref);
    Thin::with_mut(&mut thin, |thin| *thin = Default::default());
    let boxed = Thin::into_inner(thin);
}
