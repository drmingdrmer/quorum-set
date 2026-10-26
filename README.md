# quorum-set

`quorum-set` models hierarchical quorum rules with deterministic canonical IDs.
It also provides a single quorum-evaluation trait and a progress tracker for
applying those rules in consensus systems.

The `QuorumSet`, `QuorumIntersection`, and `QuorumBridge` traits and the
`VecProgress` tracker were extracted from the `quorum` and `progress` modules of
[openraft](https://github.com/databendlabs/openraft). `QuorumTree` and
`CanonicalId` are new to this crate.

The shared abstraction is `QuorumSet`: given candidate node IDs, it answers
whether those IDs form a quorum and exposes the full voter ID set. The crate
ships seven implementations:

- `BTreeSet<ID>`: a flat majority quorum.
- `Vec<BTreeSet<ID>>`: a joint quorum, accepted only when every member config accepts.
- `ThresholdQuorumSet<ID>`: a flat quorum of at least `quorum_size` IDs.
- `Joint<Q>`: a joint of rules of any type, accepted only when every rule accepts.
- `QuorumTree<ID>`: a hierarchical quorum built from nested `Node` values.
- `BothQuorums<R, W>`: a read rule and a write rule, accepted only when both accept.
- `ReadOrBoth<R, W>`: either the read rule alone or `BothQuorums<R, W>`, chosen at runtime.

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
a duplicate child node, on a `quorum_size` of `0`, or on a `quorum_size` larger
than the number of children.

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

Two read quorums may be disjoint, and so may two write quorums. So neither rule
alone can decide an election: two candidates could win disjoint quorums in the
same term. `BothQuorums` accepts an ID set only when both the read rule and the
write rule accept it. Each accepted set is then both a read quorum and a write
quorum. So with cross-intersection, any two accepted sets intersect:

```rust
use quorum_set::{BothQuorums, Node, QuorumSet, QuorumTree, verify_intersection};

fn all_of(ids: [u64; 2]) -> Node<u64> {
    Node::Subtree(QuorumTree::new(2, ids.map(Node::Id)).unwrap())
}

// Nodes form a 2x2 grid, `1 2` over `3 4`.
// A read quorum holds a row, and a write quorum holds a column.
let read_quorum = QuorumTree::new(1, [all_of([1, 2]), all_of([3, 4])]).unwrap();
let write_quorum = QuorumTree::new(1, [all_of([1, 3]), all_of([2, 4])]).unwrap();

// The two rows are disjoint, but every row intersects every column.
assert!(!verify_intersection(&read_quorum, &read_quorum));
assert!(verify_intersection(&read_quorum, &write_quorum));

let election_quorum = BothQuorums::new(read_quorum, write_quorum);

// A row alone is not enough; a row plus a column is.
assert!(!election_quorum.is_quorum([1, 2].iter()));
assert!(election_quorum.is_quorum([1, 2, 3].iter()));

// Any two sets that `election_quorum` accepts intersect.
assert!(verify_intersection(&election_quorum, &election_quorum));
```

This election argument holds when two candidates of one term are not ordered,
as in Raft. With totally ordered ballots, as in Paxos, two leaders always differ
in ballot. A voter also rejects entries from a ballot smaller than one it
granted. So read quorums need to intersect only write quorums, as
[Flexible Paxos](https://arxiv.org/abs/1608.06696) shows, and the read rule
alone can elect a leader. `ReadOrBoth` holds either the read rule alone or
`BothQuorums`, so a protocol can pick its election rule at runtime:

```rust
use quorum_set::{BothQuorums, Node, QuorumSet, QuorumTree, ReadOrBoth};

fn all_of(ids: [u64; 2]) -> Node<u64> {
    Node::Subtree(QuorumTree::new(2, ids.map(Node::Id)).unwrap())
}

fn election_quorum<R, W>(totally_ordered: bool, read: R, write: W) -> ReadOrBoth<R, W> {
    if totally_ordered {
        ReadOrBoth::Read(read)
    } else {
        ReadOrBoth::Both(BothQuorums::new(read, write))
    }
}

// The same 2x2 grid: a read quorum holds a row, and a write quorum holds a column.
let read_quorum = QuorumTree::new(1, [all_of([1, 2]), all_of([3, 4])]).unwrap();
let write_quorum = QuorumTree::new(1, [all_of([1, 3]), all_of([2, 4])]).unwrap();

let paxos = election_quorum(true, read_quorum.clone(), write_quorum.clone());
let raft = election_quorum(false, read_quorum, write_quorum);

// A row alone elects a leader with totally ordered ballots, but not in Raft.
assert!(paxos.is_quorum([1, 2].iter()));
assert!(!raft.is_quorum([1, 2].iter()));
```

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

`QuorumIntersection` checks whether every quorum of one rule intersects every
quorum of another. `QuorumBridge::joint_with` combines rules into a joint that
requires a quorum of each input.

`intersects_with` returns `Option<bool>` because an implementation may use a
condition that is sufficient but not necessary. `Some(true)` proves the
relation, `Some(false)` proves a disjoint quorum pair, and `None` proves
neither. The flat-config implementation is exact and always answers `Some`.
The joint-config implementation returns that exact answer for two uniform
joints, proves `Some(true)` when some config of one joint intersects some
config of the other, gives the exact answer when either joint is empty, and
answers `None` for every other pair. A read/write membership change can join the
old and new read rules and separately join the old and new write rules with
`joint_with`. When each membership has read/write quorum intersection, the
joint keeps cross-intersection with both memberships. `verify_intersection`
computes the exact relation for any two quorum sets, including a read tree against a write
tree. The `ThresholdQuorumSet` implementation is exact like the flat-config one,
and `Joint` answers by the joint-config rules for rules of any type.

## Canonical IDs

Every `QuorumTree` has a deterministic canonical ID through the `CanonicalId`
trait. Tree equality and ordering are based on this canonical ID.

User-defined node IDs may implement `CanonicalId`. When those IDs are embedded
as `Node::Id`, this crate escapes short canonical IDs and hashes long canonical
IDs, so tree IDs remain unambiguous and bounded.

## License

Licensed under either of the [MIT License](https://opensource.org/licenses/MIT)
or the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0), at
your choosing.
