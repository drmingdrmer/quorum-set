use std::collections::BTreeSet;

use crate::Node;
use crate::QuorumTree;
use crate::quorum::quorum_set::QuorumSet;

/// Majority quorum implementation for a flat voter set.
///
/// A candidate set is accepted when it contains more than half of the IDs in
/// this [`BTreeSet`]. An empty [`BTreeSet`] accepts no candidate set, unlike
/// an empty joint config ([`Vec<BTreeSet>`]), which accepts every candidate
/// set.
impl<ID> QuorumSet for BTreeSet<ID>
where ID: PartialOrd + Ord + Clone
{
    type Id = ID;
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: Iterator<Item = &'a ID> + Clone,
    {
        let mut count = 0;
        let limit = self.len();
        for id in self {
            if ids.clone().any(|candidate| candidate == id) {
                count += 2;
                if count > limit {
                    return true;
                }
            }
        }
        false
    }

    fn ids(&self) -> Self::Iter {
        self.clone().into_iter()
    }
}

/// Joint quorum implementation for multiple flat voter sets.
///
/// A candidate set is accepted only when it is a majority quorum in every
/// member config. [`QuorumSet::ids`] returns the deduplicated union of all
/// config IDs. An empty [`Vec`] accepts every candidate set: with no member
/// config, the requirement is vacuously satisfied.
impl<NID> QuorumSet for Vec<BTreeSet<NID>>
where NID: PartialOrd + Ord + Clone
{
    type Id = NID;
    type Iter = std::collections::btree_set::IntoIter<NID>;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        NID: 'a,
        I: Iterator<Item = &'a NID> + Clone,
    {
        for config in self {
            if !config.is_quorum(ids.clone()) {
                return false;
            }
        }
        true
    }

    fn ids(&self) -> Self::Iter {
        let mut ids = BTreeSet::new();
        for config in self {
            ids.extend(config.iter().cloned());
        }
        ids.into_iter()
    }
}

/// Hierarchical quorum implementation for [`QuorumTree`].
///
/// Evaluation follows the tree structure. `ids()` returns all leaf IDs once,
/// even when the same node appears in more than one subtree.
impl<ID> QuorumSet for QuorumTree<ID>
where ID: Ord + Clone
{
    type Id = ID;
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: Iterator<Item = &'a ID> + Clone,
    {
        self.spec.is_quorum(ids)
    }

    fn ids(&self) -> Self::Iter {
        let mut ids = BTreeSet::new();
        collect_tree_ids(self, &mut ids);
        ids.into_iter()
    }
}

fn collect_tree_ids<ID>(tree: &QuorumTree<ID>, ids: &mut BTreeSet<ID>)
where ID: Ord + Clone {
    for node in tree.children() {
        match node {
            Node::Id(id) => {
                ids.insert(id.clone());
            }
            Node::Subtree(subtree) => {
                collect_tree_ids(subtree, ids);
            }
        }
    }
}
