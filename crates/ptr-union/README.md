Pointer union types the size of a pointer
by storing the tag in the alignment bits.

## Changelist

### 2.2.0
#### Added

- `Union8` and `Union16` types. You can now use up to the four low bits for a
  tagged pointer.
- `Union*` now has safe `new_$variant` and `try_deref` methods, and `Enum*` has
  a `try_pack` method. These do a dynamic alignment check that the pointer is
  sufficiently aligned, and allow `Union` types to be used without `unsafe`.

### 2.1.0
#### Added

- `fn Union*::as_untagged_ptr(&self) -> ErasedPtr`

### 2.0.0
#### Fixes

- Union types now drop their contents properly. (Whoops!)
  This is a breaking change for two main reasons:

  - Trait bounds must be added to the union type to have them on `Drop`
  - `Copy` can no longer be provided for unions of `Copy` pointers,
    because `Drop` and `Copy` are mutually exclusive.

  I also took this opportunity to clean up the builder proof API a little,
  as the previous shape was more difficult to use than intended.

#### How did this happen

We do run the test suite for this crate under [miri]. In fact, miri is how the
leak was diagnosed and ensured to be fixed. However, the test suite previously
did not actually attempt to drop any pointer union, and the author thought that
it did. This combination let the lack of a `Drop` impl be overlooked.

#### Using previous versions

In short, you're better off not. However, if you must for some reason,
make sure that any time you drop a pointer union, you call `unpack`.
This will ensure that the inner types are properly dropped instead of leaking.

## Related Crates

- [`erasable`](https://lib.rs/crates/erasable): Erase pointers of their concrete type.
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
