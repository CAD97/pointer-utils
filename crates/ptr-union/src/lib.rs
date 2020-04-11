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
        mem::{self, ManuallyDrop},
        ops::Deref,
    },
    erasable::{ErasablePtr, ErasedPtr},
};

const MASK_2: usize = 0b01;
const MASK_4: usize = 0b11;
const MASK_A: usize = 0b00;
const MASK_B: usize = 0b01;
const MASK_C: usize = 0b10;
const MASK_D: usize = 0b11;

fn check(ptr: ErasedPtr, mask: usize, value: usize) -> bool {
    debug_assert_eq!(value & mask, value);
    (ptr.as_ptr() as usize & mask) == value
}

fn mask(ptr: ErasedPtr, mask: usize, value: usize) -> ErasedPtr {
    debug_assert_eq!(value & mask, value);
    debug_assert!(check(ptr, mask, 0));
    let high = ptr.as_ptr() as usize & !mask;
    let low = value & mask;
    unsafe { ErasedPtr::new_unchecked((high | low) as *mut _) }
}

fn unmask(ptr: ErasedPtr, mask: usize, value: usize) -> ErasedPtr {
    debug_assert!(check(ptr, mask, value));
    unsafe { ErasedPtr::new_unchecked((ptr.as_ptr() as usize & !mask) as *mut _) }
}

// NB: If we get strong const generics eventually, we can provide a safe constructor!
//     fn new() where A: AlignedPtr<N>, B: AlignedPtr<N>, C: AlignedPtr<N>, D: AlignedPtr<N> {}
//     unsafe trait AlignedPtr<const N: usize>: ErasablePtr {}
//     unsafe impl<P: ErasablePtr, const N: usize> AlignedPtr<N> for P where <{align_of::<P::Deref>() >= N}> {}
/// A builder for pointer unions that enforces correct alignment.
///
/// Currently, because there is no way to generically talk about alignment in the type system, this
/// requires the use of `unsafe` for the programmer to assert that the types are properly aligned.
/// For that, you get all of the unsafe pointer-wrangling for pointer-sized pointer unions.
///
/// In the future, with sufficiently advanced const generics, it might be possible to avoid this.
pub struct UnionBuilder<U> {
    private: PhantomData<U>,
}

impl<U> Copy for UnionBuilder<U> {}
impl<U> Clone for UnionBuilder<U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<U> fmt::Debug for UnionBuilder<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UnionBuilder")
            .field(&format_args!("{}", core::any::type_name::<U>()))
            .finish()
    }
}

impl<A, B> UnionBuilder<Union2<A, B>> {
    /// Assert that creating pointer unions of these two types is safe.
    ///
    /// # Safety
    ///
    /// Both `A` and `B` pointer types must be [erasable](`ErasablePtr`),
    /// and their erased form must align to at least `u16` (`#[repr(align(2))]`).
    ///
    /// # Examples
    ///
    /// Sound:
    ///
    /// ```rust
    /// # use ptr_union::*;
    /// # unsafe {
    /// UnionBuilder::<Union2<Box<u16>, &u32>>::new();
    /// # }
    /// ```
    ///
    /// Unsound:
    ///
    /// ```rust
    /// # use ptr_union::*;
    /// # unsafe {
    /// UnionBuilder::<Union2<Box<u16>, &u8>>::new();
    /// # }
    /// ```
    pub const unsafe fn new() -> Self {
        UnionBuilder {
            private: PhantomData,
        }
    }
}

impl<A, B, C, D> UnionBuilder<Union4<A, B, C, D>> {
    /// Assert that creating pointer unions of these four types is safe.
    ///
    /// # Safety
    ///
    /// The `A`, `B`, `C`, and `D` pointer types must be [erasable](`ErasablePtr`),
    /// and their erased form must align to at least `u32` (`#[repr(align(4))]`).
    ///
    /// # Examples
    ///
    /// Sound:
    ///
    /// ```rust
    /// # use ptr_union::*; use std::sync::Arc;
    /// # unsafe {
    /// UnionBuilder::<Union4<Box<u32>, &u32, Arc<u32>, Arc<u64>>>::new();
    /// # }
    /// ```
    ///
    /// Unsound:
    ///
    /// ```rust
    /// # use ptr_union::*; use std::sync::Arc;
    /// # unsafe {
    /// UnionBuilder::<Union4<Box<u16>, &u16, Arc<u16>, Arc<u64>>>::new();
    /// # }
    /// ```
    pub const unsafe fn new() -> Self {
        UnionBuilder {
            private: PhantomData,
        }
    }
}

/// A pointer union of two pointer types.
///
/// This is a tagged union of two pointer types such as `Box<T>`, `Arc<T>`, or `&T` that is only as
/// big as a pointer. This is accomplished by storing the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `u16` (`#[repr(align(2))]`).
/// This is enforced through the use of [`UnionBuilder`].
pub struct Union2<A, B> {
    raw: ErasedPtr,
    a: PhantomData<A>,
    b: PhantomData<B>,
}

/// A pointer union of three or four pointer types.
///
/// This is a tagged union of two pointer types such as `Box<T>`, `Arc<T>`, or `&T` that is only as
/// big as a pointer. This is accomplished by storing the tag in the alignment bits of the pointer.
///
/// As such, the pointer must be aligned to at least `u32` (`#[repr(align(4))]`).
/// This is enforced through the use of [`UnionBuilder`].
///
/// The last type may be omitted for a three pointer union.
/// Just specify a `Union4<A, B, C>` instead of `Union4<A, B, C, D>`.
/// The used `NeverPtr` type will be changed to `!` once it stabilizes.
/// This will not be considered a breaking change.
pub struct Union4<A, B, C, D = NeverPtr> {
    raw: ErasedPtr,
    a: PhantomData<A>,
    b: PhantomData<B>,
    c: PhantomData<C>,
    d: PhantomData<D>,
}

impl<A: ErasablePtr, B: ErasablePtr> UnionBuilder<Union2<A, B>> {
    /// Construct a union at this variant.
    pub fn a(self, a: A) -> Union2<A, B> {
        Union2 {
            raw: mask(A::erase(a), MASK_2, MASK_A),
            a: PhantomData,
            b: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn b(self, b: B) -> Union2<A, B> {
        Union2 {
            raw: mask(B::erase(b), MASK_2, MASK_B),
            a: PhantomData,
            b: PhantomData,
        }
    }
}

impl<A: ErasablePtr, B: ErasablePtr, C: ErasablePtr, D: ErasablePtr>
    UnionBuilder<Union4<A, B, C, D>>
{
    /// Construct a union at this variant.
    pub fn a(self, a: A) -> Union4<A, B, C, D> {
        Union4 {
            raw: mask(A::erase(a), MASK_4, MASK_A),
            a: PhantomData,
            b: PhantomData,
            c: PhantomData,
            d: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn b(self, b: B) -> Union4<A, B, C, D> {
        Union4 {
            raw: mask(B::erase(b), MASK_4, MASK_B),
            a: PhantomData,
            b: PhantomData,
            c: PhantomData,
            d: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn c(self, c: C) -> Union4<A, B, C, D> {
        Union4 {
            raw: mask(C::erase(c), MASK_4, MASK_C),
            a: PhantomData,
            b: PhantomData,
            c: PhantomData,
            d: PhantomData,
        }
    }

    /// Construct a union at this variant.
    pub fn d(self, d: D) -> Union4<A, B, C, D> {
        Union4 {
            raw: mask(D::erase(d), MASK_4, MASK_D),
            a: PhantomData,
            b: PhantomData,
            c: PhantomData,
            d: PhantomData,
        }
    }
}

macro_rules! union_methods {
    ($Union:ident: $mask:ident $([$a:ident $A:ident])*) => {
        impl<$($A: ErasablePtr),*> $Union<$($A),*> {
            $(
                paste::item! {
                    /// Check if the union is this variant.
                    pub fn [<is_ $a>](&self) -> bool {
                        check(self.raw, $mask, [<MASK_ $A>])
                    }
                }
                paste::item! {
                    /// Extract this variant from the union.
                    ///
                    /// Returns the union on error.
                    pub fn [<into_ $a>](self) -> Result<$A, Self> {
                        if self.[<is_ $a>]() {
                            unsafe { Ok($A::unerase(unmask(self.raw, $mask, [<MASK_ $A>]))) }
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
                                let $a = ManuallyDrop::new($A::unerase(unmask(self.raw, $mask, [<MASK_ $A>])));
                                Some(f(&$a))
                            }
                        } else {
                            None
                        }
                    }
                }
                paste::item! {
                    /// Get a reference to this variant's target.
                    pub fn $a(&self) -> Option<&$A::Target>
                    where
                        $A: Deref,
                    {
                        self.[<with_ $a>](|$a| unsafe { erase_lt(&**$a) })
                    }
                }
                paste::item! {
                    /// Clone this variant out of the union.
                    pub fn [<clone_ $a>](&self) -> Option<$A>
                    where
                        $A: Clone,
                    {
                        self.[<with_ $a>](|$a| $a.clone() )
                    }
                }
                paste::item! {
                    /// Copy this variant out of the union.
                    pub fn [<copy_ $a>](&self) -> Option<$A>
                    where
                        $A: Copy,
                    {
                        self.[<with_ $a>](|$a| *$a)
                    }
                }
            )*

            /// NB: Not safe generally!
            /// Because `as_deref_unchecked` only requires the actual reference is aligned.
            /// So it can be used for overaligned &T where not all &T are aligned enough.
            unsafe fn builder(&self) -> UnionBuilder<Self> {
                UnionBuilder::<Self>::new()
            }

            /// Check if two `Union`s are the same variant
            /// and point to the same value
            /// (not that the values compare as equal).
            pub fn ptr_eq(&self, other: &Self) -> bool {
                self.raw == other.raw
            }

            /// Dereference the current pointer.
            pub fn as_deref<'a>(
                &'a self,
                proof: UnionBuilder<$Union<$(&'a $A::Target),*>>,
            ) -> $Union<$(&'a $A::Target),*>
            where
                $($A: Deref,)*
                $(&'a $A::Target: ErasablePtr,)*
            {
                $(if let Some($a) = self.$a() {
                    proof.$a($a)
                } else)* {
                    unsafe { unreachable_unchecked() }
                }
            }
        }

        unsafe impl<$($A: ErasablePtr),*> ErasablePtr for $Union<$($A),*> {
            fn erase(this: Self) -> ErasedPtr {
                ManuallyDrop::new(this).raw
            }

            unsafe fn unerase(this: ErasedPtr) -> Self {
                Self {
                    raw: this,
                    $($a: PhantomData,)*
                }
            }
        }
    };
}

union_methods!(Union2: MASK_2 [a A] [b B]);
union_methods!(Union4: MASK_4 [a A] [b B] [c C] [d D]);

impl<A: ErasablePtr, B: ErasablePtr> Union2<A, B> {
    // NB: This function is defined outside the macro for specialized documentation.
    /// Dereference the current pointer.
    ///
    /// # Safety
    ///
    /// The reference produced by dereferencing must align to at least `u16` (2 bytes).
    pub unsafe fn as_deref_unchecked<'a>(&'a self) -> Union2<&'a A::Target, &'a B::Target>
    where
        A: Deref,
        B: Deref,
        &'a A::Target: ErasablePtr,
        &'a B::Target: ErasablePtr,
    {
        self.as_deref(UnionBuilder::<Union2<_, _>>::new())
    }

    /// Unpack this union into an enum.
    pub fn unpack(self) -> Enum2<A, B> {
        Err(self)
            .or_else(|this| this.into_a().map(Enum2::A))
            .or_else(|this| this.into_b().map(Enum2::B))
            .unwrap_or_else(|_| unsafe { unreachable_unchecked() })
    }
}

impl<A: ErasablePtr, B: ErasablePtr, C: ErasablePtr, D: ErasablePtr> Union4<A, B, C, D> {
    // NB: This function is defined outside the macro for specialized documentation.
    /// Dereference the current pointer.
    ///
    /// # Safety
    ///
    /// The reference produced by dereferencing must align to at least `u32` (4 bytes).
    pub unsafe fn as_deref_unchecked<'a>(
        &'a self,
    ) -> Union4<&'a A::Target, &'a B::Target, &'a C::Target, &'a D::Target>
    where
        A: Deref,
        B: Deref,
        C: Deref,
        D: Deref,
        &'a A::Target: ErasablePtr,
        &'a B::Target: ErasablePtr,
        &'a C::Target: ErasablePtr,
        &'a D::Target: ErasablePtr,
    {
        self.as_deref(UnionBuilder::<Union4<_, _, _, _>>::new())
    }

    /// Unpack this union into an enum.
    pub fn unpack(self) -> Enum4<A, B, C, D> {
        Err(self)
            .or_else(|this| this.into_a().map(Enum4::A))
            .or_else(|this| this.into_b().map(Enum4::B))
            .or_else(|this| this.into_c().map(Enum4::C))
            .or_else(|this| this.into_d().map(Enum4::D))
            .unwrap_or_else(|_| unsafe { unreachable_unchecked() })
    }
}

macro_rules! union_traits {
    ($Union:ident $([$a:ident $A:ident])*) => {
        impl<$($A: ErasablePtr),*> fmt::Debug for $Union<$($A),*>
        where
            $($A: fmt::Debug,)*
        {
            paste::item! {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    None
                        $(.or_else(|| self.[<with_ $a>](|$a| f
                            .debug_tuple(concat!(stringify!($Union), "::", stringify!($A)))
                            .field($a)
                            .finish()
                        )))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }

        impl<$($A: ErasablePtr),*> Copy for $Union<$($A),*> where $($A: Copy,)* {}
        impl<$($A: ErasablePtr),*> Clone for $Union<$($A),*>
        where
            $($A: Clone,)*
        {
            paste::item! {
                fn clone(&self) -> Self {
                    let builder = unsafe { self.builder() };
                    None
                        $(.or_else(|| self.[<clone_ $a>]().map(|$a| builder.$a($a))))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }

        impl<$($A: ErasablePtr,)*> Eq for $Union<$($A),*> where $($A: Eq,)* {}
        impl<$($A: ErasablePtr),*> PartialEq for $Union<$($A),*>
        where
            $($A: PartialEq,)*
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
        where
            $($A: Hash,)*
        {
            paste::item! {
                fn hash<H>(&self, state: &mut H)
                where
                    H: hash::Hasher
                {
                    None
                        $(.or_else(|| self.[<with_ $a>](|$a| $a.hash(state))))*
                        .unwrap_or_else(|| unsafe { unreachable_unchecked() })
                }
            }
        }
    };
}

union_traits!(Union2 [a A] [b B]);
union_traits!(Union4 [a A] [b B] [c C] [d D]);

unsafe fn erase_lt<'a, 'b, T: ?Sized>(r: &'a T) -> &'b T {
    mem::transmute(r)
}

#[cfg(not(has_never))]
use priv_in_pub::NeverPtr;
#[cfg(not(has_never))]
mod priv_in_pub {
    use super::*;

    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub enum NeverPtr {}
    unsafe impl ErasablePtr for NeverPtr {
        fn erase(_: Self) -> ErasedPtr {
            unreachable!()
        }
        unsafe fn unerase(_: ErasedPtr) -> Self {
            unreachable!()
        }
    }
}
#[cfg(has_never)]
pub type NeverPtr = !;

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
pub enum Enum4<A, B, C, D = NeverPtr> {
    A(A),
    B(B),
    C(C),
    D(D),
}

impl<A: ErasablePtr, B: ErasablePtr> Enum2<A, B> {
    /// Pack this loose enum into a pointer union.
    pub fn pack(self, proof: UnionBuilder<Union2<A, B>>) -> Union2<A, B> {
        match self {
            Enum2::A(a) => proof.a(a),
            Enum2::B(b) => proof.b(b),
        }
    }

    /// Pack this loose enum into a pointer union.
    ///
    /// # Safety
    ///
    /// The used pointer must align to at least `u16` (`#[repr(align(2))]`).
    pub unsafe fn pack_unchecked(self) -> Union2<A, B> {
        self.pack(UnionBuilder::<Union2<A, B>>::new())
    }
}

impl<A: ErasablePtr, B: ErasablePtr, C: ErasablePtr, D: ErasablePtr> Enum4<A, B, C, D> {
    /// Pack this loose enum into a pointer union.
    pub fn pack(self, proof: UnionBuilder<Union4<A, B, C, D>>) -> Union4<A, B, C, D> {
        match self {
            Enum4::A(a) => proof.a(a),
            Enum4::B(b) => proof.b(b),
            Enum4::C(c) => proof.c(c),
            Enum4::D(d) => proof.d(d),
        }
    }

    /// Pack this loose enum into a pointer union.
    ///
    /// # Safety
    ///
    /// The used pointer must align to at least `u32` (`#[repr(align(4))]`).
    pub unsafe fn pack_unchecked(self) -> Union4<A, B, C, D> {
        self.pack(UnionBuilder::<Union4<A, B, C, D>>::new())
    }
}
