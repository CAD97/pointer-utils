# Announcing pointer-utils

Recently, I (@CAD97) have ended up writing a lot of raw-pointer heavy code.
Who would've thought, optimizing tree structures requires dirty pointer work.

This repository holds a number of utilities based around pointers
that I found myself re-implementing in different contexts.

## [erasable](https://lib.rs/crates/erasable)

The core crate of this collection.
`erasable` revolves around manipulation of type-erased pointers,
and exposes four key APIs:

- [`ErasedPtr`](https://docs.rs/erasable/1.0.0/erasable/type.ErasedPtr.html): a type-erased pointer.
  This is just a type alias for `ptr::NonNull` to an unknown type, roughly equivalent to C's `void*`.
  An erased pointer is most easily created via [`fn erase`](https://docs.rs/erasable/1.0.0/erasable/fn.erase.html).
- [`Erasable`](https://docs.rs/erasable/1.0.0/erasable/trait.Erasable.html): a type that can be recovered behind a type-erased pointer.
  This is implemented for all `Sized` types and can be unsafely implemented for unsized types that know their own pointer metadata.
  It is the two reflexive APIs of `fn(ptr::NonNull<Self>) -> ErasedPtr` and `fn(ErasedPtr) -> ptr::NonNull<Self>`.
- [`ErasablePtr`](https://docs.rs/erasable/1.0.0/erasable/trait.ErasablePtr.html): a pointer type that can be erased.
  
