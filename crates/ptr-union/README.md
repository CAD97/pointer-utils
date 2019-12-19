Pointer union types the size of a pointer
by storing the tag in the alignment bits.

## Minimum Supported Rust Version

We require a minimum Rust version of 1.41.0.
This is for an adjustment of local trait impl checking.

The minimum version of Rust will only be incremented with minor version bumps,
not patch version bumps, and will be deliberate and clearly noted in change notes.

## Related Crates

- [`erasable`](https://lib.rs/crates/erasable): Erase pointers of their concrete type.
- [`rc-borrow`](https://lib.rs/crates/rc-borrow): Borrowed forms of `Rc` and `Arc`.
- [`rc-box`](https://lib.rs/crates/rc-box): Known unique forms of `Rc` and `Arc`.

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
