Erase pointers of their concrete type and store type-erased pointers.

This is roughly equivalent to C's `void*`, but it does not use `libc::c_void`.

There are two main useful reasons to type erase pointers in Rust:

- Removing viral generics from internal implementation details.
  If the internals truly don't care about the stored type,
  treating it opaquely reduces monomorphization cost
  both to the author and the compiler.
- Thin pointers to `?Sized` types. If an unsized type stores its metadata inline,
  then it can implement [`Erasable`](https://cad97.github.io/pointer-utils/erasable/trait.Erasable.html)
  and be used behind type-erased pointers.
  The type erased pointer does not have to carry the metadata,
  and the fat pointer can be recovered from the inline metadata.
  We provide the [`Thin`](https://cad97.github.io/pointer-utils/erasable/struct.Thin.html)
  wrapper type to provide thin pointer types.

## Changelist

### 1.2.0
#### Added
- `impl ErasablePtr for Thin<P>`: the obvious impl; `Thin` is erased internally
- `Thin::ptr_eq`: easily compare pointer equality of two thin pointers

### 1.1.0
#### Breaking changes

Unfortunately, a subtle problem with the requirements of `Erasable::unerase`
on implementors was found, and needed to be corrected in this version.

It has always been the intent that a pointer of any provenance should roundtrip
through `Erasable::erase`/`unerase` without any impact on provenance validity
of any pointer. Unfortunately, in 1.0.0, `Eraseable::unerase` explicitly suggested
the use of temporary shared references (`&_`) in its implementation, which do not
hold up this desired semantics. To quote `Erasable::unerase`'s new documentation:

> No references to the pointee _at all_ should be created,
> as their mere temporary existence may impact the validity and
> usable provenance of other pointers to the same location.
>
> Creating a shared reference sounds on the surface like it should be ok.
> After all, you have a known-valid pointer to your type, and you can
> borrow from whatever pointer was erased. However, in the face of raw
> pointers with a shared mutable provenance, this is problematic.
> If a write to the pointee location even potentially races with any
> invocation of `unerase`, and it creates a reference to the location,
> we have immediate undefined behavior for writing behind a shared ref.
>
> The root issue is that there may be external synchronization that this
> implementation has no way of knowing about. An implementation of this
> trait must only read the mimimum amount of data required to re-type the
> pointer, and must do so with a raw pointer read, or, if and only if
> there is a known `UnsafeCell` point (such as an atomic), a reference to
> that `UnsafeCell` point and the safe API of that `UnsafeCell` point.

For more information around the discovery of this issue see
[this Twitter thread](https://twitter.com/CAD97_/status/1231057021623054336).

To make this update easier, we additionally added `const Erasable::ACK_1_1_0: bool`.
This value defaults to `false`, but _must_ be overriden to `true` in new
implementations of `Erasable` as an explicit acknowledgement of the new requirement.
To make finding impls that do not do so, the `ERASABLE_ENFORCE_1_1_0_SEMANTICS`
environment variable can be set to remove the default implementation,
explicitly breaking any implementor that did not override its value.

If you have a use of `unerase` that would be made unsound by `unerase` creating
a shared reference, then it is recommended to assert the value of `ACK_1_1_0`
to guard against old impls that may create a shared reference. Also, please tell
me what it is, because I have not been able to construct a reasonable example
that would be made unsound by this hole, just a theoretical attack vector.
Similarly, if you can show that this isn't required, get in touch.
I'd love to remove this hack if it turns out unnecessary after all
(but I think I'm confident ruling out that possibility).

## Related Crates

- [`ptr-union`](https://lib.rs/crates/ptr-union): Pointer unions the size of a pointer.
- [`rc-borrow`](https://lib.rs/crates/rc-borrow): Borrowed forms of `Rc` and `Arc`.
- [`rc-box`](https://lib.rs/crates/rc-box): Known unique forms of `Rc` and `Arc`.
- [`slice-dst`](https://lib.rs/crates/slice-dst): Support for custom slice-based DSTs.

## Minimum Supported Rust Version

We require a minimum Rust version of 1.41.0.
This is for an adjustment of local trait impl checking.

Minimum version support is only guaranteed with minimal version resolution
(`-Z minimal-versions`/`--minimal-versions`) due to how dependencies are handled.
The minimum version of Rust will only be incremented with minor version bumps,
not patch version bumps, and will be deliberate and clearly noted in change notes.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE/APACHE](../../LICENSE/APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE/MIT](../../LICENSE/MIT) or http://opensource.org/licenses/MIT)

at your option.

If you are a highly paid worker at any company that prioritises profit over
people, you can still use this crate. I simply wish you will unionise and push
back against the obsession for growth, control, and power that is rampant in
your workplace. Please take a stand against the horrible working conditions
they inflict on your lesser paid colleagues, and more generally their
disrespect for the very human rights they claim to fight for.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
