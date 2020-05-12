use {
    either::{Either, Left, Right},
    erasable::{ErasablePtr, ErasedPtr, Thin},
    std::ops::{Deref, DerefMut},
};

struct MeanestDerefInTheWest {
    evil: Box<Either<Box<usize>, Box<usize>>>,
}

impl MeanestDerefInTheWest {
    fn new() -> Self {
        MeanestDerefInTheWest {
            evil: Box::new(Left(Box::new(0))),
        }
    }
}

unsafe impl ErasablePtr for MeanestDerefInTheWest {
    fn erase(this: Self) -> ErasedPtr {
        ErasablePtr::erase(this.evil)
    }
    unsafe fn unerase(this: ErasedPtr) -> Self {
        MeanestDerefInTheWest {
            evil: ErasablePtr::unerase(this),
        }
    }
}

impl Deref for MeanestDerefInTheWest {
    type Target = usize;

    fn deref(&self) -> &usize {
        self.evil.as_ref().as_ref().into_inner().as_ref() // LUL
    }
}

// this is the interesting/evil bit
impl DerefMut for MeanestDerefInTheWest {
    fn deref_mut(&mut self) -> &mut usize {
        let val = **self;
        if self.evil.is_left() {
            self.evil = Box::new(Right(Box::new(val + 1)));
        } else {
            self.evil = Box::new(Left(Box::new(val + 1)));
        }
        self.evil.as_mut().as_mut().into_inner().as_mut() // LUL
    }
}

#[test]
fn meanest_deref_in_the_west() {
    let mut mean = Thin::from(MeanestDerefInTheWest::new());
    for _ in 0..10 {
        mean.deref_mut();
    }
    assert_eq!(*mean, 10);
}

#[test]
fn panic_with_mut() {
    let mut b: Thin<Box<u8>> = Box::new(0).into();
    std::panic::catch_unwind(move || Thin::with_mut(&mut b, |_| panic!())).unwrap_err();
}
