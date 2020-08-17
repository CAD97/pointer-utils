Support for custom slice-based DSTs.

By handling allocation manually, we can manually allocate the `Box` for a custom DST.
So long as the size lines up with what it should be, once the metadata is created,
Rust actually already handles the DSTs it already supports perfectly well, safely!
Setting them up is the hard part, which this crate handles for you.

## Examples

We have a tree structure! Each node holds some data and its children array.
In normal Rust, you would probably typically implement it something like this:

```rust
struct Node {
    data: &'static str,
    children: Vec<Arc<Node>>,
}

let a = Node { data: "a", children: vec![] };
let b = Node { data: "b", children: vec![] };
let c = Node { data: "c", children: vec![] };
let abc = Node { data: "abc", children: vec![a.into(), b.into(), c.into()] };
```

With this setup, the memory layout looks vaguely like the following diagram:

```text
                                             +--------------+
                                             |Node          |
                                       +---->|data: "a"     |
+------------+    +---------------+    |     |children: none|
|Node        |    |Vec<Arc<Node>> |    |     +--------------+
|data: "abc" |    |[0]: +--------------+     |Node          |
|children: +----->|[1]: +------------------->|data: "b"     |
+------------+    |[2]: +--------------+     |children: none|
                  +---------------|    |     +--------------+
                                       |     |Node          |
                                       +---->|data: "c"     |
                                             |children: none|
                                             +--------------+
```

With this crate, however, the children array can be stored inline with the node's data:

```rust
#[derive(SliceDst)]
#[repr(C)]
#[slice_dst(new_from_iter)]
struct Node {
    data: &'static str,
    slice: [Arc<Node>],
}

// the generated constructor is deliberately awkward; create a wrapper to expose
let a = Node::new_from_iter(("a",), vec![]);
let b = Node::new_from_iter(("b",), vec![]);
let c = Node::new_from_iter(("c",), vec![]);
let abc: Arc<Node> = Node::new_from_iter(("abc",), vec![a, b, c]);
```

```text
                        +----------+
                        |Node      |
+------------+    +---->|data: "a" |
|Node        |    |     |slice: [] |
|data: "abc" |    |     +----------+
|slice: [0]: +----+     |Node      |
|       [1]: +--------->|data: "b" |
|       [2]: +----+     |slice: [] |
+------------+    |     +----------+
                  |     |Node      |
                  +---->|data: "c" |
                        |slice: [] |
                        +----------+
```

The exact times you will want to use this rather than just standard types varries.
This is mostly useful when space optimization is very important.
This is still useful when using an arena: it reduces the allocations in the arena
in exchange for moving node payloads to the heap alongside the children array.

## Changelist

### 1.6.0
#### Additions

- Added the ability to derive `SliceDst`. The derive implements the trait and
  optionally provides safe constructors for simple custom slice DSTS. This is
  gated by a default-on feature, `derive`.

#### MSRV

- MSRV is increased from 1.41.0 to 1.44.0.

### 1.5.0
#### Additions

- Added `SliceWithHeader::from_slice`, which is a specialization of `_::new`
  for slices of `Copy` types that can avoid some bookkeeping overhead.

### 1.4.0
#### Additions

- Added the `TryAllocSliceDst`, a fallible analogue to `AllocSliceDst`.

### 1.3.0
#### Additions

- Added a `StrWithHeader` type, counterpart to `SliceWithHeader`, but with a `str`.

### 1.2.0
#### Soundness Fixes
- `alloc_slice_dst`(`_in`) accidentally improperly used [`slice::from_raw_parts_mut`]
  instead of [`ptr::slice_from_raw_parts_mut`], even when the latter is available on
  Rust version `^1.42.0`. For more information, see [the fix PR][#45].

- The buildscript checking for [`ptr::slice_from_raw_parts_mut`]'s stabilization was
  bugged, and always failed, leaving code using [`slice::from_raw_parts_mut`] instead.
  For technical details, see [the fix PR][#47].

These fixes only have an impact if you are using Rust 1.42 or higher, and do not
cause any known miscompilations (nor even fail miri). However, out of an
abundance of caution, we have still seen fit to yank all versions of slice-dst
in the 1.1 line, and urge you to upgrade to 1.2 as soon as possible.

  [`slice::from_raw_parts_mut`]: <https://doc.rust-lang.org/std/slice/fn.from_raw_parts_mut.html>
  [`ptr::slice_from_raw_parts_mut`]: <https://doc.rust-lang.org/std/ptr/fn.slice_from_raw_parts_mut.html>
  [#45]: <https://github.com/CAD97/pointer-utils/pull/45>
  [#47]: <https://github.com/CAD97/pointer-utils/pull/47>

#### Improvements
- Previously, construction of a thin slice DST would leak the allocated memory if
  a panic occurred during construction. [#44] fixes most cases to clean up properly.

  [#44]: <https://github.com/CAD97/pointer-utils/pull/44>

### 1.1.0
#### Soundness Fixes
- `alloc_slice_dst`(`_in`) now properly support zero-sized types.

## Related Crates

- [`erasable`](https://lib.rs/crates/erasable): Erase pointers of their concrete type.
- [`ptr-union`](https://lib.rs/crates/ptr-union): Pointer unions the size of a pointer.
- [`rc-borrow`](https://lib.rs/crates/rc-borrow): Borrowed forms of `Rc` and `Arc`.
- [`rc-box`](https://lib.rs/crates/rc-box): Known unique forms of `Rc` and `Arc`.

## Minimum Supported Rust Version


The `derive` feature (default) requires a minimum Rust version of 1.44.0.
This is for `Layout` manipulation methods stabilized in this version.

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

