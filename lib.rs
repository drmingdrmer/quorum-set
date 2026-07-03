#![doc = include_str!("README.md")]
#![warn(missing_docs)]

use std::cmp::Ordering;
use std::fmt;

pub use canonical_id::CanonicalId;
pub use quorum_node::QuorumNode;

mod canonical_id;
mod impl_display;
mod quorum_node;
mod quorum_tree_spec;

use quorum_tree_spec::QuorumTreeSpec;

/// A single quorum rule represented as a tree.
///
/// `QuorumTree` models one side of a quorum configuration. A consensus
/// implementation should usually build one tree for read quorums and another
/// tree for write quorums, then ensure every read quorum intersects every write
/// quorum.
///
/// A child node is selected when it is either an included node ID or a nested
/// quorum tree whose own quorum rule is satisfied.
///
/// # Invariants
///
/// - A [`QuorumTree`] represents exactly one quorum rule.
/// - Child nodes are stored in a `BTreeSet`, so duplicate nodes are removed and traversal order is
///   deterministic according to [`QuorumNode`] ordering.
/// - Equality and ordering for [`QuorumTree`] are based only on its canonical ID.
/// - Canonical IDs are stable identifiers. [`std::fmt::Display`] is human-readable output and is
///   not a serialization format.
#[derive(Clone, Debug)]
pub struct QuorumTree<ID>
where ID: Ord
{
    spec: QuorumTreeSpec<ID>,

    canonical_id: String,
}

impl<ID> QuorumTree<ID>
where ID: Ord
{
    /// Builds a quorum tree from a quorum number and child nodes.
    ///
    /// `quorum_num` is the number of child nodes that must be selected for this
    /// tree to be selected. A child can be a single ID or another quorum tree.
    ///
    /// Duplicate child nodes are removed. If `quorum_num` is `0`, every input
    /// satisfies the tree. If `quorum_num` is greater than the number of unique
    /// child nodes, no input can satisfy the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use quorum_tree::{QuorumNode, QuorumTree};
    ///
    /// let tree = QuorumTree::new(2, [
    ///     QuorumNode::Id(1),
    ///     QuorumNode::Id(2),
    ///     QuorumNode::Id(3),
    /// ]);
    ///
    /// assert!(tree.is_quorum(&[1, 2]));
    /// assert!(!tree.is_quorum(&[1]));
    /// ```
    pub fn new(quorum_num: u64, nodes: impl IntoIterator<Item = QuorumNode<ID>>) -> Self
    where ID: CanonicalId {
        let spec = QuorumTreeSpec::new(quorum_num, nodes);
        let canonical_id = spec.canonical_id();
        Self { spec, canonical_id }
    }

    /// Returns the number of selected child nodes required to satisfy this
    /// tree.
    pub fn quorum_num(&self) -> u64 {
        self.spec.quorum_num()
    }

    /// Returns whether `ids` satisfy this quorum tree.
    ///
    /// Each child node can contribute at most one selected child. Repeating the
    /// same ID in `ids` does not increase the selected count.
    pub fn is_quorum(&self, ids: &[ID]) -> bool {
        self.spec.is_quorum(ids)
    }
}

/// Equality and ordering are decided solely by `canonical_id`.
///
/// `new()` is the sole constructor and `canonical_id` is the cached canonical
/// representation of the `QuorumTreeSpec`. A single string comparison keeps
/// `BTreeSet` operations cheap; no recursive structural comparison is needed.
impl<ID> PartialEq for QuorumTree<ID>
where ID: Ord
{
    fn eq(&self, other: &Self) -> bool {
        self.canonical_id == other.canonical_id
    }
}

impl<ID> Eq for QuorumTree<ID> where ID: Ord {}

impl<ID> PartialOrd for QuorumTree<ID>
where ID: Ord
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<ID> Ord for QuorumTree<ID>
where ID: Ord
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.canonical_id.cmp(&other.canonical_id)
    }
}

impl<ID> CanonicalId for QuorumTree<ID>
where ID: Ord + CanonicalId
{
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        write!(f, "{}", self.canonical_id)
    }
}
