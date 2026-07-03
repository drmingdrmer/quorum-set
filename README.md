# quorum-tree

`quorum-tree` models hierarchical quorum rules with deterministic canonical IDs.

The main type is `QuorumTree`. A tree contains child nodes, and each child is
either a node ID or another `QuorumTree`. The tree is satisfied when at least
`quorum_size` children are selected.

This crate is designed for consensus systems that need separate read and write
quorum rules. A complete quorum configuration should use two trees:

- one `QuorumTree` for read quorums
- one `QuorumTree` for write quorums

The required property is cross-intersection:

- every read quorum must intersect with every write quorum
- two read quorums do not necessarily need to intersect with each other
- two write quorums do not necessarily need to intersect with each other

Because the intersection requirement is between the read tree and the write
tree, `quorum_size` does not always need to be a majority of the tree's nodes. A
read tree can require fewer than half of the nodes if the write tree is defined
so every write quorum still intersects every possible read quorum.

This crate checks whether a set of node IDs satisfies one tree. It does not
prove that a read tree and a write tree have the required cross-intersection
property. The caller is responsible for building compatible read and write
trees.

## Usage

```rust
use quorum_tree::{Node, QuorumTree};

let read_quorum = QuorumTree::new(1, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]);

let write_quorum = QuorumTree::new(3, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]);

assert!(read_quorum.is_quorum(&[1]));
assert!(!write_quorum.is_quorum(&[1, 2]));
assert!(write_quorum.is_quorum(&[1, 2, 3]));
```

In this example, a read quorum can be a single node, and the only write quorum
contains all nodes. Therefore every read quorum intersects with every write
quorum, even though read quorums do not intersect with each other.

The simpler setup is to use the same majority tree for reads and writes:

```rust
use quorum_tree::{Node, QuorumTree};

let read_quorum = QuorumTree::new(2, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]);

let write_quorum = read_quorum.clone();

assert!(read_quorum.is_quorum(&[1, 2]));
assert!(write_quorum.is_quorum(&[2, 3]));
```

For a flat tree, setting `quorum_size` to at least `nodes.len() / 2 + 1` gives
the usual majority quorum rule.

## Hierarchical Quorums

Nested trees model grouped layouts. This example selects a write quorum only
when both groups have a local majority:

```rust
use quorum_tree::{Node, QuorumTree};

fn id(i: u64) -> Node<u64> {
    Node::Id(i)
}

fn group(nodes: [u64; 3]) -> Node<u64> {
    Node::Subtree(QuorumTree::new(2, nodes.into_iter().map(id)))
}

let write_quorum = QuorumTree::new(2, [
    group([1, 2, 3]),
    group([4, 5, 6]),
]);

assert!(write_quorum.is_quorum(&[1, 2, 4, 5]));
assert!(!write_quorum.is_quorum(&[1, 2, 4]));
```

## Canonical IDs

Every `QuorumTree` has a deterministic canonical ID through the `CanonicalId`
trait. Tree equality and ordering are based on this canonical ID.

User-defined node IDs may implement `CanonicalId`. When those IDs are embedded
as `Node::Id`, this crate escapes short canonical IDs and hashes long canonical
IDs, so tree IDs remain unambiguous and bounded.
