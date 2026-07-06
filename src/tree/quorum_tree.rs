use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

use super::quorum_tree_spec::QuorumTreeSpec;
use crate::CanonicalId;
use crate::Node;
use crate::QuorumTreeError;

mod impl_display;

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
/// - Child nodes are unique and traversal order is deterministic according to [`Node`] ordering.
/// - `quorum_size` never exceeds the number of child nodes: construction rejects duplicate children
///   and unsatisfiable quorum sizes.
/// - Equality and ordering for [`QuorumTree`] are based only on its canonical ID.
/// - Canonical IDs are stable identifiers. [`std::fmt::Display`] is human-readable output and is
///   not a serialization format.
#[derive(Clone, Debug)]
pub struct QuorumTree<ID>
where ID: Ord
{
    pub(crate) spec: QuorumTreeSpec<ID>,

    canonical_id: String,
}

impl<ID> QuorumTree<ID>
where ID: Ord
{
    /// Builds a quorum tree from a quorum size and child nodes.
    ///
    /// `quorum_size` is the number of child nodes that must be selected for this
    /// tree to be selected. A child can be a single ID or another quorum tree.
    /// If `quorum_size` is `0`, every input satisfies the tree.
    ///
    /// # Errors
    ///
    /// - [`QuorumTreeError::DuplicateChild`]: a child was given more than once. A duplicate child
    ///   is always an upstream bug; a tree never contains the same child twice.
    /// - [`QuorumTreeError::UnsatisfiableQuorum`]: `quorum_size` exceeds the number of child nodes,
    ///   so no input could satisfy the tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use quorum_set::{Node, QuorumSet, QuorumTree, QuorumTreeError};
    ///
    /// let tree = QuorumTree::new(2, [
    ///     Node::Id(1),
    ///     Node::Id(2),
    ///     Node::Id(3),
    /// ]).unwrap();
    ///
    /// assert!(tree.is_quorum([1, 2].iter()));
    /// assert!(!tree.is_quorum([1].iter()));
    ///
    /// let err = QuorumTree::new(1, [Node::Id(1), Node::Id(1)]).unwrap_err();
    /// assert_eq!(
    ///     QuorumTreeError::DuplicateChild { canonical_id: "Id=1".to_string() },
    ///     err
    /// );
    /// ```
    pub fn new(
        quorum_size: u64,
        nodes: impl IntoIterator<Item = Node<ID>>,
    ) -> Result<Self, QuorumTreeError>
    where
        ID: CanonicalId,
    {
        let spec = QuorumTreeSpec::new(quorum_size, nodes)?;
        let canonical_id = spec.canonical_id();
        Ok(Self { spec, canonical_id })
    }

    /// Returns the number of selected child nodes required to satisfy this
    /// tree.
    pub fn quorum_size(&self) -> u64 {
        self.spec.quorum_size()
    }

    /// Returns this tree's child nodes in canonical order.
    ///
    /// Children are unique: construction rejects duplicate child nodes.
    pub fn children(&self) -> impl Iterator<Item = &Node<ID>> {
        self.spec.children()
    }

    /// Returns the canonical ID of this tree.
    ///
    /// The canonical ID decides equality and ordering. It is computed once at
    /// construction, so this accessor does not allocate.
    pub fn canonical_id(&self) -> &str {
        &self.canonical_id
    }
}

/// Equality and ordering are decided solely by `canonical_id`.
///
/// `new()` is the sole constructor and `canonical_id` is the cached canonical
/// representation of the quorum tree spec. A single string comparison keeps
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

/// Hashing is based solely on `canonical_id`, consistent with equality.
impl<ID> Hash for QuorumTree<ID>
where ID: Ord
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical_id.hash(state);
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::Node;
    use crate::QuorumTree;
    use crate::QuorumTreeError;

    fn id(i: u64) -> Node<u64> {
        Node::Id(i)
    }

    #[test]
    fn test_eq_ignores_construction_order() {
        let a = QuorumTree::new(2, [id(1), id(2), id(3)]).unwrap();
        let b = QuorumTree::new(2, [id(3), id(1), id(2)]).unwrap();
        let c = QuorumTree::new(3, [id(1), id(2), id(3)]).unwrap();

        assert_eq!(a, b);
        assert_eq!(a.canonical_id(), b.canonical_id());
        assert_ne!(a, c);
    }

    #[test]
    fn test_hash_is_consistent_with_eq() {
        let a = QuorumTree::new(2, [id(1), id(2), id(3)]).unwrap();
        let b = QuorumTree::new(2, [id(3), id(2), id(1)]).unwrap();
        let c = QuorumTree::new(3, [id(1), id(2), id(3)]).unwrap();

        let set: HashSet<QuorumTree<u64>> = [a.clone(), b.clone(), c.clone()].into_iter().collect();

        assert_eq!(HashSet::from([a, c]), set);
    }

    #[test]
    fn test_canonical_id_accessor() {
        let tree = QuorumTree::new(2, [id(3), id(1), id(2)]).unwrap();

        assert_eq!("2/(Id=1,Id=2,Id=3)", tree.canonical_id());
    }

    #[test]
    fn test_children_are_sorted() {
        let tree = QuorumTree::new(2, [id(3), id(1), id(2)]).unwrap();

        assert_eq!(
            vec![id(1), id(2), id(3)],
            tree.children().cloned().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_new_rejects_duplicate_child() {
        let err = QuorumTree::new(2, [id(1), id(2), id(2)]).unwrap_err();

        assert_eq!(
            QuorumTreeError::DuplicateChild {
                canonical_id: "Id=2".to_string()
            },
            err
        );

        let sub = QuorumTree::new(1, [id(1), id(2)]).unwrap();
        let err = QuorumTree::new(2, [Node::Subtree(sub.clone()), Node::Subtree(sub)]).unwrap_err();

        assert_eq!(
            QuorumTreeError::DuplicateChild {
                canonical_id: "Subtree=1/(Id=1,Id=2)".to_string(),
            },
            err
        );
    }

    #[test]
    fn test_new_rejects_unsatisfiable_quorum() {
        let err = QuorumTree::new(3, [id(1), id(2)]).unwrap_err();

        assert_eq!(
            QuorumTreeError::UnsatisfiableQuorum {
                quorum_size: 3,
                num_children: 2
            },
            err
        );
    }
}
