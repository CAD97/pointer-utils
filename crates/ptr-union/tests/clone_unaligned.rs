//! This is a regression test for https://github.com/CAD97/pointer-utils/issues/89
//!
//! The idea here is to have a Box like pointer which we can control the alignment for to
//! ensure that the tests are stable (doesn't depend on allocator shenanigans).

use std::{ptr::NonNull, sync::atomic::AtomicUsize};

use ptr_union::Union8;

#[derive(Debug)]
struct MyBox {
    ptr: NonNull<u8>,
}

fn ptr_dangling_at<T>(addr: usize) -> *mut T {
    #[cfg(not(has_strict_provenance))]
    {
        addr as _
    }
    #[cfg(has_strict_provenance)]
    {
        std::ptr::without_provenance_mut(addr)
    }
}

// SAFETY:
// * MyBox doesn't have any shared mutability
// * the address of the returned pointer doesn't depend on the address of MyBox
// * MyBox doesn't implement Deref
unsafe impl erasable::ErasablePtr for MyBox {
    fn erase(this: Self) -> erasable::ErasedPtr {
        this.ptr.cast()
    }

    unsafe fn unerase(this: erasable::ErasedPtr) -> Self {
        Self { ptr: this.cast() }
    }
}

static OFFSET: AtomicUsize = AtomicUsize::new(8);

impl MyBox {
    fn new() -> Self {
        let offset = OFFSET.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        MyBox {
            ptr: NonNull::new(ptr_dangling_at(offset)).unwrap(),
        }
    }
}

impl Clone for MyBox {
    fn clone(&self) -> Self {
        Self::new()
    }
}

type Union = Union8<
    MyBox,
    NonNull<u8>,
    NonNull<u8>,
    NonNull<u8>,
    NonNull<u8>,
    NonNull<u8>,
    NonNull<u8>,
    NonNull<u8>,
>;

#[test]
#[allow(clippy::redundant_clone)]
#[should_panic = "but the cloned pointer wasn't sufficiently aligned"]
fn test_clone_unaligned() {
    let bx = MyBox::new();
    // this can't fail since the first `MyBox` is created at address 8, which is aligned to 8 bytes
    let x = Union::new_a(bx).unwrap();

    // this clone should panic, since the next `MyBox` is created at address 9, which is not aligned to 8 bytes
    let _y = x.clone();
}
