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
  It should be noted that this trait has some tricky `unsafe` guarantees that it must provide, but it basically encodes
  "pointer `Deref` semantics" and the unsafe lifetime trickery that is typically sound around said smart pointer types.
- [`Thin`](https://docs.rs/erasable/1.0.0/erasable/struct.Thin.html): a pointer wrapper to store pointer types as erased thin pointers.

Erasable doesn't itself provide thin-capable unsized types, but it fully supports them when unsafely implemented.

## [slice-dst](https://lib.rs/crates/slice-dst)

This crate provides slice-based custom dynamically sized types and support for thin pointers to them.
There are two key traits in addition to that of erasable that enable this ergonomically:

- [`AllocSliceDst`](https://docs.rs/slice-dst/1.0.0/slice_dst/trait.AllocSliceDst.html), which is a trait for
  types which can allocate a slice-based DST. Basically, this is a trait for "owned smart pointer" types.
  Two functions, [`alloc_slice_dst`](https://docs.rs/slice-dst/1.0.0/slice_dst/fn.alloc_slice_dst.html) and
  [`alloc_slice_dst`](https://docs.rs/slice-dst/1.0.0/slice_dst/fn.alloc_slice_dst_in.html) are provided to
  assist in the creation of `AllocSliceDst` implementations.
- [`SliceDst`](https://docs.rs/slice-dst/1.0.0/slice_dst/trait.SliceDst.html), for actual slice-based DSTs.
  All this trait actually requires is a way to calculate the allocation layout of the type from the slice length,
  and to add on the type information onto a raw pointer coming out of the allocation.
  Support for erasing and thin pointers is added just by an implementation of `Erasable`.

Unless you need to specially pack the slice length into your structure's other data, the simpler (and safe)
solution is [`SliceWithHeader`](https://docs.rs/slice-dst/1.0.0/slice_dst/struct.SliceWithHeader.html),
which offers a type that is effectively a tuple of `for<Header, Item> (length: usize, Header, [Item])`.
It offers a single method, [`new`](https://docs.rs/slice-dst/1.0.0/slice_dst/struct.SliceWithHeader.html#method.new),
which is polymorphic over potential containers and collects an iterator directly into the target allocation<sup>†</sup>.

† At the current time, `Rc` and `Arc` do support slice-based DSTs, but must allocate first in a `Box`,
then move into the reference-counted allocation. The
[`YOLO_RC_HEAP_LAYOUT_KNOWN`](https://github.com/CAD97/pointer-utils/blob/master/crates/slice-dst/src/yolo_rc_impls.rs)
environment variable can be set to very unsafely assume it knows the heap layout of (`A`)`Rc` and do a direct allocation.
This is directly relying on implementation details of the standard library, and is not stable API.
I just think it's cool, and it shows off that this actually is theoretically supported.

## Generally useful crates

The remaining three crates are more generally directly useful than erasable and slice-dst,
and are much more simple in concept.

### [ptr-union](https://lib.rs/crates/ptr-union)

Simple, automatic unions of (`ErasablePtr`) pointer types, tagged in alignment bits.
This means that the union is just one `usize` large.
(Yes, it supports erasable DSTs, though fat pointers are not supported at this time.)
Only one invocation of `unsafe` is required per union type used,
in order to assert that the alignment requirements are met.

### [rc-borrow](https://lib.rs/crates/rc-borrow)

Borrowed forms of `Arc` and `Rc`, but with only one layer of indirection.
Basically `&T`, but guaranteed to exist behind a reference-count, so it can be upgraded to an owned pointer.

### [rc-box](https://lib.rs/crates/rc-box)

Known unique forms of `Arc` and `Rc`. This allows direct mutation of the pointer payload,
as if you were using `Box` rather than (`A`)`Rc`. This lifts to the type system the check
that (`A`)`Rc::get_mut` performs at runtime (and in turn eliminates the runtime check).

# So why should I use it?

First I should mention the main alternatvive: [triomphe](https://lib.rs/crates/triomphe).
Triomphe is its own atomic reference counting library, originally by the servo developers,
that provides an `Arc` alternative with

- no weak counts (sparing you the weak count overhead),
- strong guarantees for using it for FFI, and
- all of the nicities the pointer-utils collections provides,
  such as a thin slice/header pair and a borrowed reference count.

If this is all you need, then you should probably use triomphe, as it is more battle-tested.

However, if you want to work with the standard library types, triomphe won't work,
as it's a distinct fork of the standard library `Arc`.
This also means it doesn't benefit from improvements to the standard `Arc`'s implementation and API.
The pointer-utils collection works directly with the standard library's pointer types,
and is easily extensible to your own pointer types if necessary.

On top of this, triomphe doesn't have an equivalent API to erasable.
In my own personal experience, erasable makes working with type-erased pointers
a lot easier and less error-prone, in an already dangerous part of Rust.
To me, that alone [makes it worth it](https://youtu.be/rHIkrotSwcc).

Though, to be completely frank: if you don't know that you want something like this,
you're probably best off not using it and sticking to standard types without pointer-utils.
This is by design a power-user optimization tool, rather than a go-to utility.
