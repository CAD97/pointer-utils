Support for custom slice-based DSTs.

By handling allocation manually, we can manually allocate the `Box` for a custom DST.
So long as the size lines up with what it should be, once the metadata is created,
Rust actually already handles the DSTs it already supports perfectly well, safely!
Setting them up is the hard part, which this crate handles for you.

# Examples

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
struct Node(Arc<SliceWithHeader<&'static str, Node>>);

let a = Node(SliceWithHeader::new("a", None));
let b = Node(SliceWithHeader::new("b", None));
let c = Node(SliceWithHeader::new("c", None));
// this vec is just an easy way to get an ExactSizeIterator
let abc = Node(SliceWithHeader::new("abc", vec![a, b, c]));
```

```text
                         +-----------+
+-------------+          |Node       |
|Node         |    +---->|length: 0  |
|length: 3    |    |     |header: "a"|
|header: "abc"|    |     +-----------+
|slice: [0]: +-----+     |Node       |
|       [1]: +---------->|length: 0  |
|       [2]: +-----+     |header: "b"|
+-------------+    |     +-----------+
                   |     |Node       |
                   +---->|length: 0  |
                         |header: "c"|
                         +------------
```

The exact times you will want to use this rather than just standard types varries.
This is mostly useful when space optimization is very important.
This is still useful when using an arena: it reduces the allocations in the arena
in exchange for moving node payloads to the heap alongside the children array.
