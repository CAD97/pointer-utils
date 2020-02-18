use {
    erasable::*,
    std::{cell::Cell, ops::Deref},
};

#[test]
#[ignore = "example of unsound code"]
fn deref_before_indirection_is_unsound_example() {
    struct Why {
        inner: Box<u8>,
    }

    unsafe impl ErasablePtr for Why {
        fn erase(this: Self) -> ErasedPtr {
            ErasablePtr::erase(this.inner)
        }
        unsafe fn unerase(this: ErasedPtr) -> Self {
            Why {
                inner: ErasablePtr::unerase(this),
            }
        }
    }

    impl Deref for Why {
        type Target = Box<u8>;
        fn deref(&self) -> &Box<u8> {
            &self.inner
        }
    }

    let thin = Thin::from(Why { inner: Box::new(0) });
    #[allow(clippy::borrowed_box)]
    let _: &Box<u8> = thin.deref(); // use-after-free; cannot deref to value that does not exist
}

#[test]
#[ignore = "example of unsound code"]
fn shared_mutability_before_indirection_is_unsound_example() {
    struct Pls {
        inner: Cell<Box<u8>>,
    }

    unsafe impl ErasablePtr for Pls {
        fn erase(this: Self) -> ErasedPtr {
            ErasablePtr::erase(this.inner.into_inner())
        }
        unsafe fn unerase(this: ErasedPtr) -> Self {
            Pls {
                inner: Cell::new(ErasablePtr::unerase(this)),
            }
        }
    }

    impl Pls {
        fn mutate(&self, to: Box<u8>) {
            self.inner.set(to);
        }
    }

    let thin = Thin::from(Pls {
        inner: Cell::new(Box::new(0)),
    });
    Thin::with(&thin, |pls| pls.mutate(Box::new(1))); // drops box(0), leaks box(1)
    drop(thin); // `thin` is still Pls(Box(0)); use-after-free
}
