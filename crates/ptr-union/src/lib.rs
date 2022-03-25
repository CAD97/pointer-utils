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

const MASK_2: usize = 0b0001;
const MASK_4: usize = 0b0011;
const MASK_8: usize = 0b0111;
const MASK_16: usize = 0b1111;
const TAG_A: usize = 0b0000;
const TAG_B: usize = 0b0001;
const TAG_C: usize = 0b0010;
const TAG_D: usize = 0b0011;
const TAG_E: usize = 0b0100;
const TAG_F: usize = 0b0101;
const TAG_G: usize = 0b0110;
const TAG_H: usize = 0b0111;
const TAG_I: usize = 0b1000;
const TAG_J: usize = 0b1001;
const TAG_K: usize = 0b1010;
const TAG_L: usize = 0b1011;
const TAG_M: usize = 0b1100;
const TAG_N: usize = 0b1101;
const TAG_O: usize = 0b1110;
const TAG_P: usize = 0b1111;

// See rust-lang/rust#95228 for why these are necessary.
fn ptr_addr<T>(this: *mut T) -> usize {
    // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
    this as usize
}

fn ptr_with_addr<T>(this: *mut T, addr: usize) -> *mut T {
    // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
    //
    // In the mean-time, this operation is defined to be "as if" it was
    // a wrapping_offset, so we can emulate it as such. This should properly
    // restore pointer provenance even under today's compiler.
    let this_addr = ptr_addr(this) as isize;
    let dest_addr = addr as isize;
    let offset = dest_addr.wrapping_sub(this_addr);

    // This is the canonical desugarring of this operation
    this.cast::<u8>().wrapping_offset(offset).cast::<T>()
}

fn ptr_map_addr<T>(this: *mut T, f: impl FnOnce(usize) -> usize) -> *mut T {
    ptr_with_addr(this, f(ptr_addr(this)))
}

fn ptr_tag<T>(this: *mut T, tag: usize) -> *mut T {
    ptr_map_addr(this, |addr| addr | tag)
}

fn ptr_mask<T>(this: *mut T, mask: usize) -> *mut T {
    ptr_map_addr(this, |addr| addr & mask)
}

#[inline(always)]
fn check_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> bool {
    debug_assert_eq!(tag & mask, tag);
    ptr_addr(ptr_mask(ptr.as_ptr(), mask)) == tag
}

#[inline(always)]
fn set_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> ErasedPtr {
    debug_assert_eq!(tag & mask, tag);
    debug_assert!(check_tag(ptr, mask, 0));
    unsafe { ErasedPtr::new_unchecked(ptr_tag(ptr.as_ptr(), tag)) }
}

#[inline(always)]
fn unset_tag(ptr: ErasedPtr, mask: usize, tag: usize) -> ErasedPtr {
    debug_assert_eq!(tag & mask, tag);
    debug_assert!(check_tag(ptr, mask, tag));
    unsafe { ErasedPtr::new_unchecked(ptr_mask(ptr.as_ptr(), !mask)) }
}

#[inline(always)]
fn unset_any_tag(ptr: ErasedPtr, mask: usize) -> ErasedPtr {
    unsafe { ErasedPtr::new_unchecked(ptr_mask(ptr.as_ptr(), !mask)) }
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

/// A pointer union of eight pointer types.
///
/// This is a tagged union of eight pointer types such as `Box`, `Arc`, or `&`
/// that is only as big as a single pointer. This is accomplished by storing
/// the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `u64` (`align(8)`).
/// This is enforced through the use of [`Builder8`].
///
/// Pointers beyond the fifth may be ommitted to create smaller unions.
/// The default type, `NeverPtr`, will be an alias for `!` once it is stable.
/// This will not be considered a breaking change.
pub struct Union8<
    A: ErasablePtr,
    B: ErasablePtr,
    C: ErasablePtr,
    D: ErasablePtr,
    E: ErasablePtr,
    F: ErasablePtr = NeverPtr,
    G: ErasablePtr = NeverPtr,
    H: ErasablePtr = NeverPtr,
> {
    raw: ErasedPtr,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<Enum8<A, B, C, D, E, F, G, H>>,
}

/// A pointer union of up to sixteen pointer types.
///
/// This is a tagged union of sixteen pointer types such as `Box`, `Arc`, or `&`
/// that is only as big as a single pointer. This is accomplished by storing
/// the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `align(16)`.
/// This is enforced through the use of [`Builder16`].
///
/// Pointers beyond the ninth may be ommitted to create smaller unions.
/// The default type, `NeverPtr`, will be an alias for `!` once it is stable.
/// This will not be considered a breaking change.
pub struct Union16<
    A: ErasablePtr,
    B: ErasablePtr,
    C: ErasablePtr,
    D: ErasablePtr,
    E: ErasablePtr,
    F: ErasablePtr,
    G: ErasablePtr,
    H: ErasablePtr,
    I: ErasablePtr,
    J: ErasablePtr = NeverPtr,
    K: ErasablePtr = NeverPtr,
    L: ErasablePtr = NeverPtr,
    M: ErasablePtr = NeverPtr,
    N: ErasablePtr = NeverPtr,
    O: ErasablePtr = NeverPtr,
    P: ErasablePtr = NeverPtr,
> {
    raw: ErasedPtr,
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<Enum16<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P>>,
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

/// An unpacked version of [`Union4`].
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Enum8<A, B, C, D, E, F, G, H> {
    A(A),
    B(B),
    C(C),
    D(D),
    E(E),
    F(F),
    G(G),
    H(H),
}

/// An unpacked version of [`Union8`].
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Enum16<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P> {
    A(A),
    B(B),
    C(C),
    D(D),
    E(E),
    F(F),
    G(G),
    H(H),
    I(I),
    J(J),
    K(K),
    L(L),
    M(M),
    N(N),
    O(O),
    P(P),
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

/// A builder for [`Union8`].
///
/// An instance of this builder means that `Union8` parameterized
/// with the same type arguments are safe to construct.
pub struct Builder8<A, B, C, D, E, F = NeverPtr, G = NeverPtr, H = NeverPtr> {
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<Enum8<A, B, C, D, E, F, G, H>>,
}

/// A builder for [`Union16`].
///
/// An instance of this builder means that `Union16` parameterized
/// with the same type arguments are safe to construct.
pub struct Builder16<
    A,
    B,
    C,
    D,
    E,
    F = NeverPtr,
    G = NeverPtr,
    H = NeverPtr,
    I = NeverPtr,
    J = NeverPtr,
    K = NeverPtr,
    L = NeverPtr,
    M = NeverPtr,
    N = NeverPtr,
    O = NeverPtr,
    P = NeverPtr,
> {
    #[allow(clippy::type_complexity)]
    phantom: PhantomData<Enum16<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P>>,
}

macro_rules! impl_builder {
    ($UnionName:ident $Union:ty, $BuilderName:ident $Builder:ty: $mask:ident $([$a:ident $A:ident])*) => {
        impl<$($A),*> $Builder {
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

        impl<$($A: ErasablePtr),*> $Builder {
            paste::paste! {
                $(
                    /// Construct a union at this variant.
                    pub fn $a(self, this: $A) -> $Union {
                        $UnionName {
                            raw: set_tag($A::erase(this), $mask, [<TAG_ $A>]),
                            phantom: PhantomData,
                        }
                    }
                )*
            }
        }

        impl<$($A),*> Copy for $Builder {}
        impl<$($A),*> Clone for $Builder {
            fn clone(&self) -> Self {
                *self
            }
        }
    };
}

macro_rules! impl_union {
    ($Union:ident, $Enum:ident, $Builder:ident: $mask:ident $([$a:ident $A:ident])*) => {
        impl_builder!($Union $Union<$($A),*>, $Builder $Builder<$($A),*>: $mask $([$a $A])*);

        impl<$($A: ErasablePtr),*> $Union<$($A),*> {
            paste::paste! {
                $(
                    /// Construct a varaint of this union with a dynamic alignment check.
                    pub fn [<new_ $a>]($a: $A) -> Result<Self, $A> {
                        let $a = $A::erase($a);
                        if check_tag($a, $mask, 0) {
                            Ok($Union {
                                raw: set_tag($a, $mask, [<TAG_ $A>]),
                                phantom: PhantomData,
                            })
                        } else {
                            Err(unsafe { $A::unerase($a) })
                        }
                    }

                    /// Check if the union is this variant.
                    pub fn [<is_ $a>](&self) -> bool {
                        check_tag(self.raw, $mask, [<TAG_ $A>])
                    }

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

                    /// Get a reference to this variant's target.
                    pub fn $a(&self) -> Option<&$A::Target>
                    where $A: Deref
                    {
                        self.[<with_ $a>](|this| unsafe { erase_lt(&**this) })
                    }

                    /// Clone this variant out of the union.
                    pub fn [<clone_ $a>](&self) -> Option<$A>
                    where $A: Clone
                    {
                        self.[<with_ $a>](|this| this.clone())
                    }

                    /// Copy this variant out of the union.
                    pub fn [<copy_ $a>](&self) -> Option<$A>
                    where $A: Copy
                    {
                        self.[<with_ $a>](|this| *this)
                    }
                )*

                /// Unpack this union into an enum.
                pub fn unpack(self) -> $Enum<$($A),*> {
                    Err(self)
                        $(.or_else(|this| this.[<into_ $a>]().map($Enum::$A)))*
                        .unwrap_or_else(|_| unsafe { unreachable_unchecked() })
                }
            }

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

            /// Get the raw type-erased untagged pointer to the payload.
            pub fn as_untagged_ptr(&self) -> ErasedPtr {
                unset_any_tag(self.raw, $mask)
            }

            paste::paste! {
                /// Dereference the current pointer.
                ///
                /// Performs a dynamic alignment check on the dereferenced pointer.
                pub fn try_deref<'a>(&'a self) -> Option<$Union<$(&'a $A::Target),*>>
                where
                    $($A: Deref,)*
                    $(&'a $A::Target: ErasablePtr,)*
                {
                    $(if let Some(this) = self.$a() {
                        $Union::[<new_ $a>](this).ok()
                    } else)* {
                        None
                    }
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

            paste::paste! {
                /// Pack this loose enum into a pointer union.
                pub fn try_pack(self) -> Result<$Union<$($A),*>, Self> {
                    match self {
                        $($Enum::$A(this) => $Union::[<new_ $a>](this).map_err(Self::$A),)*
                    }
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
            paste::paste! {
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
            paste::paste! {
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
            paste::paste! {
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
            paste::paste! {
                fn hash<Hasher>(&self, state: &mut Hasher)
                where Hasher: hash::Hasher
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
impl_union!(Union8, Enum8, Builder8: MASK_8 [a A] [b B] [c C] [d D] [e E] [f F] [g G] [h H]);
impl_union!(Union16, Enum16, Builder16: MASK_16 [a A] [b B] [c C] [d D] [e E] [f F] [g G] [h H] [i I] [j J] [k K] [l L] [m M] [n N] [o O] [p P]);

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

impl<A, B, C, D, E, F, G, H> fmt::Debug for Builder8<A, B, C, D, E, F, G, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Builder8")
            .field(&format_args!(
                "Union8<{}, {}, {}, {}, {}, {}, {}, {},>",
                core::any::type_name::<A>(),
                core::any::type_name::<B>(),
                core::any::type_name::<C>(),
                core::any::type_name::<D>(),
                core::any::type_name::<E>(),
                core::any::type_name::<F>(),
                core::any::type_name::<G>(),
                core::any::type_name::<H>(),
            ))
            .finish()
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P> fmt::Debug
    for Builder16<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Builder16")
            .field(&format_args!(
                "Union16<{}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}>",
                core::any::type_name::<A>(),
                core::any::type_name::<B>(),
                core::any::type_name::<C>(),
                core::any::type_name::<D>(),
                core::any::type_name::<E>(),
                core::any::type_name::<F>(),
                core::any::type_name::<G>(),
                core::any::type_name::<H>(),
                core::any::type_name::<I>(),
                core::any::type_name::<J>(),
                core::any::type_name::<K>(),
                core::any::type_name::<L>(),
                core::any::type_name::<M>(),
                core::any::type_name::<N>(),
                core::any::type_name::<O>(),
                core::any::type_name::<P>(),
            ))
            .finish()
    }
}

unsafe fn erase_lt<'a, 'b, T: ?Sized>(r: &'a T) -> &'b T {
    &*(r as *const T)
}
