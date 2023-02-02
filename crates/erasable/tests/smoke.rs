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
    assert_eq!(erased.as_unit_ptr() as usize, ptr);
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

#[test]
fn with_fn() {
    let boxed: Box<Big> = Default::default();

    let erased: ErasedPtr = ErasablePtr::erase(boxed);

    unsafe {
        // clippy errs here:
        // warning: you seem to be trying to use `&Box<T>`. Consider using just `&T`
        //   --> crates/erasable/tests/smoke.rs:45:30
        //    |
        // 45 |         erased.with(|bigbox: &Box<Big>| {
        //    |                              ^^^^^^^^^ help: try: `&Big`
        //
        // We really need to borrow a &Box<Big> in this case because that what we constructed
        // the ErasedPtr from.
        #[allow(clippy::borrowed_box)]
        erased.with(|bigbox: &Box<Big>| {
            assert_eq!(*bigbox, Default::default());
        })
    }

    // drop it, otherwise we would leak memory here
    unsafe { <Box<Big> as ErasablePtr>::unerase(erased) };
}

#[test]
fn with_mut_fn() {
    let boxed: Box<Big> = Default::default();

    let mut erased: ErasedPtr = ErasablePtr::erase(boxed);

    unsafe {
        erased.with_mut(|bigbox: &mut Box<Big>| {
            bigbox.0[0] = 123456;
            assert_ne!(*bigbox, Default::default());
        })
    }

    // drop it, otherwise we would leak memory here
    unsafe { <Box<Big> as ErasablePtr>::unerase(erased) };
}

#[test]
fn with_mut_fn_replacethis() {
    let boxed: Box<Big> = Default::default();

    let mut erased: ErasedPtr = ErasablePtr::erase(boxed);
    let e1 = erased.as_unit_ptr();

    unsafe {
        erased.with_mut(|bigbox: &mut Box<Big>| {
            let mut newboxed: Box<Big> = Default::default();
            newboxed.0[0] = 123456;
            *bigbox = newboxed;
            assert_ne!(*bigbox, Default::default());
        })
    }

    let e2 = erased.as_unit_ptr();
    assert_ne!(e1, e2);

    // drop it, otherwise we would leak memory here
    unsafe { <Box<Big> as ErasablePtr>::unerase(erased) };
}
