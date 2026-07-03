use std::cmp::Ordering;
use std::fmt;

pub use canonical_id::CanonicalId;
pub use quorum_node::QuorumNode;
pub use quorum_tree_spec::QuorumTreeSpec;

mod canonical_id;
mod impl_display;
mod quorum_node;
mod quorum_tree_spec;

/// A quorum tree whose node can be either a node ID or a nested quorum set.
///
/// A quorum tree represents one quorum rule. Read and write quorums should be
/// represented as separate trees because they may use different node sets.
///
/// For example, a node set {A, B, C} can be a quorum set. That set {A, B, C},
/// with quorum num 2, can also be a node of another quorum set.
///
/// For example:
/// ```text
/// [quorum_num=2:[A,B,C], quorum_num=2:[D,E,F], G]
/// ```
///
/// # Invariants
///
/// - A [`QuorumTree`] represents exactly one quorum rule. Read and write quorum rules must be
///   modeled as separate trees.
/// - Child nodes are stored in a `BTreeSet`, so duplicate nodes are removed and traversal order is
///   deterministic according to [`QuorumNode`] ordering.
/// - [`QuorumTree`] caches the canonical ID built from its [`QuorumTreeSpec`]. Equality and
///   ordering for [`QuorumTree`] are based only on this canonical ID.
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
    pub fn new(quorum_num: u64, nodes: impl IntoIterator<Item = QuorumNode<ID>>) -> Self
    where ID: CanonicalId {
        let spec = QuorumTreeSpec::new(quorum_num, nodes);
        let canonical_id = spec.canonical_id();
        Self { spec, canonical_id }
    }

    pub fn quorum_num(&self) -> u64 {
        self.spec.quorum_num()
    }

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
