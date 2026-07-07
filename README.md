# quorum-set

`quorum-set` models hierarchical quorum rules with deterministic canonical IDs.
It also provides a single quorum-evaluation trait and a progress tracker for
applying those rules in consensus systems.

The shared abstraction is `QuorumSet`: given candidate node IDs, it answers
whether those IDs form a quorum and exposes the full voter ID set. The crate
ships three implementations:

- `BTreeSet<ID>`: a flat majority quorum.
- `Vec<BTreeSet<ID>>`: a joint quorum, accepted only when every member config accepts.
- `QuorumTree<ID>`: a hierarchical quorum built from nested `Node` values.

`VecProgress` is the main consumer. It tracks per-node progress and maintains
the greatest progress value accepted by the configured `QuorumSet`.
`QuorumIntersection` models the intersection relation used by membership
changes.

`QuorumTree` is the structural implementation. A tree contains child nodes, and
each child is either a node ID or another `QuorumTree`. The tree is satisfied
when at least `quorum_size` children are selected.

## Read and Write Rules

This crate is designed for consensus systems that may need separate read and
write quorum rules. A complete quorum configuration can use two trees:

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

The caller chooses the read and write rules; `verify_intersection` proves or
refutes the cross-intersection property for the chosen pair. The check is
exact for any two `QuorumSet` values but exponential in the number of distinct
voter IDs, so it is intended for validating small configurations and for
tests.

Construction rejects invalid tree rules: `QuorumTree::new` returns an error on
a duplicate child node or on a `quorum_size` larger than the number of children.

## Quorum Sets

```rust
use std::collections::BTreeSet;

use quorum_set::QuorumSet;

let majority = BTreeSet::from([1, 2, 3]);
assert!(majority.is_quorum([1, 2].iter()));
assert!(!majority.is_quorum([1].iter()));

let joint = vec![
    BTreeSet::from([1, 2, 3]),
    BTreeSet::from([3, 4, 5]),
];
assert!(joint.is_quorum([1, 3, 4].iter()));
assert!(!joint.is_quorum([1, 2].iter()));
```

The simpler flat setup is to use a majority quorum for both reads and writes.
For a flat `QuorumTree`, setting `quorum_size` to at least `nodes.len() / 2 + 1`
gives the usual majority quorum rule.

```rust
use quorum_set::{Node, QuorumSet, QuorumTree};

let read_quorum = QuorumTree::new(2, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]).unwrap();

let write_quorum = read_quorum.clone();

assert!(read_quorum.is_quorum([1, 2].iter()));
assert!(write_quorum.is_quorum([2, 3].iter()));
```

A non-majority read rule can still be valid if the write rule is strong enough
to intersect every read quorum:

```rust
use quorum_set::{Node, QuorumSet, QuorumTree, verify_intersection};

let read_quorum = QuorumTree::new(1, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]).unwrap();

let write_quorum = QuorumTree::new(3, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]).unwrap();

assert!(read_quorum.is_quorum([1].iter()));
assert!(!write_quorum.is_quorum([1, 2].iter()));
assert!(write_quorum.is_quorum([1, 2, 3].iter()));

// Every read quorum intersects every write quorum.
assert!(verify_intersection(&read_quorum, &write_quorum));

// A 2-of-3 write rule is not enough: read quorum {1} misses write quorum {2, 3}.
let weak_write = QuorumTree::new(2, [
    Node::Id(1),
    Node::Id(2),
    Node::Id(3),
]).unwrap();
assert!(!verify_intersection(&read_quorum, &weak_write));
```

In this example, a read quorum can be a single node, and the only write quorum
contains all nodes. Therefore every read quorum intersects with every write
quorum, even though read quorums do not intersect with each other.

## Hierarchical Quorums

Nested trees model grouped layouts. This example selects a write quorum only
when both groups have a local majority:

```rust
use quorum_set::{Node, QuorumSet, QuorumTree};

fn id(i: u64) -> Node<u64> {
    Node::Id(i)
}

fn group(nodes: [u64; 3]) -> Node<u64> {
    Node::Subtree(QuorumTree::new(2, nodes.into_iter().map(id)).unwrap())
}

let write_quorum = QuorumTree::new(2, [
    group([1, 2, 3]),
    group([4, 5, 6]),
]).unwrap();

assert!(write_quorum.is_quorum([1, 2, 4, 5].iter()));
assert!(!write_quorum.is_quorum([1, 2, 4].iter()));
```

`QuorumTree::ids()` returns each leaf ID once. A node may appear in more than
one subtree, but it is still one voter ID for APIs such as `VecProgress`.

## Progress Tracking

`VecProgress` works with any `QuorumSet`. It stores voter IDs from `ids()` first,
then learners, and updates the quorum-accepted value as node progress advances.

```rust
use std::collections::BTreeSet;

use quorum_set::{IdVal, VecProgress};

let voters = BTreeSet::from([1, 2, 3]);
let mut progress = VecProgress::<IdVal<u64, u64>, _>::new(
    voters,
    [],
    IdVal::new_default,
);

assert_eq!(Some(&0), progress.update_progress(&1, 5));
assert_eq!(Some(&5), progress.update_progress(&2, 5));
assert_eq!(&5, progress.quorum_accepted());
```

## Quorum Intersection

Consensus membership changes need every quorum in one membership to intersect
every quorum in the next. `QuorumIntersection` checks that relation for joint
configurations, and `QuorumBridge` builds the intermediate joint config used
when moving between memberships.

`intersects_with` may be conservative: `true` guarantees the relation, while
`false` can mean the implementation cannot prove it, in which case moving
through a joint bridge is the safe path. `verify_intersection` computes the
exact relation for any two quorum sets, including a read tree against a write
tree.

## Canonical IDs

Every `QuorumTree` has a deterministic canonical ID through the `CanonicalId`
trait. Tree equality and ordering are based on this canonical ID.

User-defined node IDs may implement `CanonicalId`. When those IDs are embedded
as `Node::Id`, this crate escapes short canonical IDs and hashes long canonical
IDs, so tree IDs remain unambiguous and bounded.
