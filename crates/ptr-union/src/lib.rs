//! Pointer union types the size of a pointer
//! by storing the tag in the alignment bits.

#![warn(missing_docs, missing_debug_implementations)]
#![no_std]

use {
    core::{
        fmt,
        hash::{self, Hash},
        hint::unreachable_unchecked,
        marker::PhantomData,
        mem::ManuallyDrop,
        ops::Deref,
        ptr,
    },
    erasable::{ErasablePtr, ErasedPtr},
};

const MASK_2: usize = 0b01;
const MASK_4: usize = 0b11;
const TAG_A: usize = 0b00;
const TAG_B: usize = 0b01;
const TAG_C: usize = 0b10;
const TAG_D: usize = 0b11;

#[inline(always)]
fn check_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> bool {
    debug_assert_eq!(tag & mask, tag);
    (ptr.as_ptr() as usize & mask) == tag
}

#[inline(always)]
fn set_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> ErasedPtr {
    debug_assert_eq!(tag & mask, tag);
    debug_assert!(check_tag(ptr, mask, 0));
    unsafe { ErasedPtr::new_unchecked((ptr.as_ptr() as usize | tag) as *mut _) }
}

#[inline(always)]
fn unset_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> ErasedPtr {
    debug_assert_eq!(tag & mask, tag);
    debug_assert!(check_tag(ptr, mask, tag));
    unsafe { ErasedPtr::new_unchecked((ptr.as_ptr() as usize & !mask) as *mut _) }
}

#[cfg(has_never)]
pub type NeverPtr = !;
#[cfg(not(has_never))]
use never_ptr::NeverPtr;
#[cfg(not(has_never))]
mod never_ptr {
    use super::*;
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub enum NeverPtr {}
    unsafe impl ErasablePtr for NeverPtr {
        #[inline]
        fn erase(this: Self) -> ErasedPtr {
            match this {}
        }
        #[inline]
        unsafe fn unerase(_this: ErasedPtr) -> Self {
            unreachable!()
        }
    }
}

/// A pointer union of two pointer types.
///
/// This is a tagged union of two pointer types such as `Box`, `Arc`, or `&`
/// that is only as big as a single pointer. This is accomplished by storing
/// the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `u16` (`align(2)`).
/// This is enforced through the use of [`Builder2`].
pub struct Union2<A: ErasablePtr, B: ErasablePtr> {
    raw: ErasedPtr,
    phantom: PhantomData<Enum2<A, B>>,
}

/// A pointer union of four pointer types.
///
/// This is a tagged union of four pointer types such as `Box`, `Arc`, or `&`
/// that is only as big as a single pointer. This is accomplished by storing
/// the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `u32` (`align(4)`).
/// This is enforced through the use of [`Builder4`].
///
/// The fourth pointer type may be omitted to create a three pointer union.
/// The default type, `NeverPtr`, will be an alias for `!` once it is stable.
/// This will not be considered a breaking change.
pub struct Union4<A: ErasablePtr, B: ErasablePtr, C: ErasablePtr, D: ErasablePtr = NeverPtr> {
    raw: ErasedPtr,
    phantom: PhantomData<Enum4<A, B, C, D>>,
}

/// An unpacked version of [`Union2`].
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Enum2<A, B> {
    A(A),
    B(B),
}

/// An unpacked version of [`Union4`].
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Enum4<A, B, C, D> {
    A(A),
    B(B),
    C(C),
    D(D),
}

/// A builder for [`Union2`].
///
/// An instance of this builder means that `Union2` parameterized
/// with the same type arguments are safe to construct.
pub struct Builder2<A, B> {
    phantom: PhantomData<Enum2<A, B>>,
}

/// A builder for [`Union4`].
///
/// An instance of this builder means that `Union4` parameterized
/// with the same type arguments are safe to construct.
pub struct Builder4<A, B, C, D = NeverPtr> {
    phantom: PhantomData<Enum4<A, B, C, D>>,
}

macro_rules! impl_union {
    ($Union:ident, $Enum:ident, $Builder:ident: $mask:ident $([$a:ident $A:ident])*) => {
        impl<$($A),*> $Builder<$($A),*> {
            /// Assert that creating pointer unions of these types is safe.
            ///
            /// # Safety
            ///
            /// The pointer types must be [erasable](`ErasablePtr`), and their
            /// alignment must meet the requirements of the target union type.
            pub const unsafe fn new_unchecked() -> Self {
                Self { phantom: PhantomData }
            }
        }

        impl<$($A: ErasablePtr),*> $Union<$($A),*> {
            $(
                paste::item! {
                    /// Check if the union is this variant.
                    pub fn [<is_ $a>](&self) -> bool {
                        check_tag(self.raw, $mask, [<TAG_ $A>])
                    }
                }
                paste::item! {
                    /// Extract this variant from the union.
                    ///
                    /// Returns the union on error.
                    pub fn [<into_ $a>](self) -> Result<$A, Self> {
                        if self.[<is_ $a>]() {
                            let this = ManuallyDrop::new(self);
                            unsafe { Ok($A::unerase(unset_tag(this.raw, $mask, [<TAG_ $A>]))) }
                        } else {
                            Err(self)
                        }
                    }
                }
                paste::item! {
                    /// Run a closure with this variant.
                    pub fn [<with_ $a>]<R>(&self, f: impl FnOnce(&$A) -> R) -> Option<R> {
                        if self.[<is_ $a>]() {
                            unsafe {
                                let this = ManuallyDrop::new($A::unerase(unset_tag(self.raw, $mask, [<TAG_ $A>])));
                                Some(f(&this))
                            }
                        } else {
                            None
                        }
                    }
                }
                paste::item! {
                    /// Get a reference to this variant's target.
                    pub fn $a(&self) -> Option<&$A::Target>
                    where $A: Deref
                    {
                        self.[<with_ $a>](|this| unsafe { erase_lt(&**this) })
                    }
                }
                paste::item! {
                    /// Clone this variant out of the union.
                    pub fn [<clone_ $a>](&self) -> Option<$A>
                    where $A: Clone
                    {
                        self.[<with_ $a>](|this| this.clone())
                    }
                }
                paste::item! {
                    /// Copy this variant out of the union.
                    pub fn [<copy_ $a>](&self) -> Option<$A>
                    where $A: Copy
                    {
                        self.[<with_ $a>](|this| *this)
                    }
                }
            )*

            /// Check if two unions are the same variant and point to
            /// the same value (not that the values compare as equal).
            pub fn ptr_eq(&self, other: &Self) -> bool {
                self.raw == other.raw
            }

            /// Dereference the current pointer.
            pub fn as_deref<'a>(
                &'a self,
                builder: $Builder<$(&'a $A::Target),*>
            ) -> $Union<$(&'a $A::Target),*>
            where
                $($A: Deref,)*
                $(&'a $A::Target: ErasablePtr,)*
            {
                $(if let Some(this) = self.$a() {
                    builder.$a(this)
                } else)* {
                    unsafe { unreachable_unchecked() }
                }
            }

            /// Dereference the current pointer.
            ///
            /// # Safety
            ///
            /// The reference produced must be properly aligned. Note that only
            /// the actually produced reference is restricted, not the result
            /// of dereferencing any of the other types in this union.
            pub unsafe fn as_deref_unchecked<'a>(&'a self) -> $Union<$(&'a $A::Target),*>
            where
                $($A: Deref,)*
                $(&'a $A::Target: ErasablePtr,)*
            {
                self.as_deref($Builder::new_unchecked())
            }

            paste::item! {
                /// Unpack this union into an enum.
                pub fn unpack(self) -> $Enum<$($A),*> {
                    Err(self)
                        $(.or_else(|this| this.[<into_ $a>]().map($Enum::$A)))*
                        .unwrap_or_else(|_| unsafe { unreachable_unchecked() })
                }
            }
        }

        impl<$($A: ErasablePtr),*> $Enum<$($A),*> {
            /// Pack this loose enum into a pointer union.
            pub fn pack(self, builder: $Builder<$($A),*>) -> $Union<$($A),*> {
                match self {
                    $($Enum::$A(this) => builder.$a(this),)*
                }
            }

            /// Pack this loose enum into a pointer union.
            ///
            /// # Safety
            ///
            /// The pointer packed must be properly aligned. Note that only
            /// the actually packed pointer is restricted, not any other
            /// pointer type involved in this definition.
            pub unsafe fn pack_unchecked(self) -> $Union<$($A),*> {
                self.pack($Builder::new_unchecked())
            }
        }

        unsafe impl<$($A: ErasablePtr),*> ErasablePtr for $Union<$($A),*> {
            fn erase(this: Self) -> ErasedPtr {
                ManuallyDrop::new(this).raw
            }

            unsafe fn unerase(this: ErasedPtr) -> Self {
                Self {
                    raw: this,
                    phantom: PhantomData,
                }
            }
        }

        impl<$($A: ErasablePtr),*> Drop for $Union<$($A),*> {
            fn drop(&mut self) {
                unsafe { drop(ptr::read(self).unpack()) }
            }
        }

        impl<$($A: ErasablePtr),*> fmt::Debug for $Union<$($A),*>
        where $($A: fmt::Debug),*
        {
            paste::item! {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    None
                        $(.or_else(|| self.[<with_ $a>](|this| f
                            .debug_tuple(stringify!($A))
                            .field(this)
                            .finish()
                        )))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }

        impl<$($A: ErasablePtr),*> Clone for $Union<$($A),*>
        where $($A: Clone),*
        {
            paste::item! {
                fn clone(&self) -> Self {
                    let builder = unsafe { <$Builder<$($A,)*>>::new_unchecked() };
                    None
                        $(.or_else(|| self.[<clone_ $a>]().map(|this| builder.$a(this))))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }

        impl<$($A: ErasablePtr,)*> Eq for $Union<$($A),*> where $($A: Eq,)* {}
        impl<$($A: ErasablePtr),*> PartialEq for $Union<$($A),*>
        where $($A: PartialEq),*
        {
            paste::item! {
                fn eq(&self, other: &Self) -> bool {
                    None
                        $(.or_else(|| self.[<with_ $a>](|this|
                            other.[<with_ $a>](|that|
                                this == that
                            ).unwrap_or(false)
                        )))*
                        .unwrap_or(false)
                }
            }
        }

        impl<$($A: ErasablePtr,)*> Hash for $Union<$($A),*>
        where $($A: Hash),*
        {
            paste::item! {
                fn hash<H>(&self, state: &mut H)
                where H: hash::Hasher
                {
                    None
                        $(.or_else(|| self.[<with_ $a>](|this| this.hash(state))))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }

        unsafe impl<$($A: ErasablePtr,)*> Send for $Union<$($A),*> where $($A: Send),* {}
        unsafe impl<$($A: ErasablePtr,)*> Sync for $Union<$($A),*> where $($A: Sync),* {}
    };
}

impl_union!(Union2, Enum2, Builder2: MASK_2 [a A] [b B]);
impl_union!(Union4, Enum4, Builder4: MASK_4 [a A] [b B] [c C] [d D]);

impl<A: ErasablePtr, B: ErasablePtr> Builder2<A, B> {
    /// Construct a union at this variant.
    pub fn a(self, this: A) -> Union2<A, B> {
        Union2 {
            raw: set_tag(A::erase(this), MASK_2, TAG_A),
            phantom: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn b(self, this: B) -> Union2<A, B> {
        Union2 {
            raw: set_tag(B::erase(this), MASK_2, TAG_B),
            phantom: PhantomData,
        }
    }
}

impl<A: ErasablePtr, B: ErasablePtr, C: ErasablePtr, D: ErasablePtr> Builder4<A, B, C, D> {
    /// Construct a union at this variant.
    pub fn a(self, this: A) -> Union4<A, B, C, D> {
        Union4 {
            raw: set_tag(A::erase(this), MASK_4, TAG_A),
            phantom: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn b(self, this: B) -> Union4<A, B, C, D> {
        Union4 {
            raw: set_tag(B::erase(this), MASK_4, TAG_B),
            phantom: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn c(self, this: C) -> Union4<A, B, C, D> {
        Union4 {
            raw: set_tag(C::erase(this), MASK_4, TAG_C),
            phantom: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn d(self, this: D) -> Union4<A, B, C, D> {
        Union4 {
            raw: set_tag(D::erase(this), MASK_4, TAG_D),
            phantom: PhantomData,
        }
    }
}

impl<A, B> fmt::Debug for Builder2<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Builder2")
            .field(&format_args!(
                "Union2<{}, {}>",
                core::any::type_name::<A>(),
                core::any::type_name::<B>(),
            ))
            .finish()
    }
}

impl<A, B> Copy for Builder2<A, B> {}
impl<A, B> Clone for Builder2<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B, C, D> fmt::Debug for Builder4<A, B, C, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Builder4")
            .field(&format_args!(
                "Union4<{}, {}, {}, {}>",
                core::any::type_name::<A>(),
                core::any::type_name::<B>(),
                core::any::type_name::<C>(),
                core::any::type_name::<D>(),
            ))
            .finish()
    }
}

impl<A, B, C, D> Copy for Builder4<A, B, C, D> {}
impl<A, B, C, D> Clone for Builder4<A, B, C, D> {
    fn clone(&self) -> Self {
        *self
    }
}

unsafe fn erase_lt<'a, 'b, T: ?Sized>(r: &'a T) -> &'b T {
    &*(r as *const T)
}
