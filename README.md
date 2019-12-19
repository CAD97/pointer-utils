# Pointer utility crates

- [`erasable`](crates/erasable):
  Erase pointers of their concrete type.
- [`rc-borrow`](crates/rc-borrow):
  Borrowed forms of `Rc` and `Arc`.
- [`rc-box`](crates/rc-box):
  Known unique forms of `Rc` and `Arc`.
- [`ptr-union`](crates/ptr-union):
  Pointer unions the size of a pointer.

## Why not [triomphe](https://crates.io/crates/triomphe)?

Triomphe is a great atomic reference counting library!
The main difference between triomphe and these utilities is that
triomphe implements a new `Arc` type that doesn't support weak references
(and as such does not have to pay the cost of handling potential weak references),
whereas these pointer utilities use the standard library's reference counting types.
If you need to work with standard library `Arc`/`Rc`, triomphe won't work for you.

If you want a more battle-tested library by the servo developers, use triomphe.
If you want small, self-contained extensions to the standard library types,
use these pointer utilities.

Additionally, triomphe only supports atomic reference counting.
We provide support for both `Arc` and `Rc`, as well as `Box` where applicable.

## Minimum Supported Rust Version

We require a minimum Rust version of 1.41.0.
This is for an adjustment of local trait impl checking.

The minimum version of Rust will only be incremented with minor version bumps,
not patch version bumps, and will be deliberate and clearly noted in change notes.
