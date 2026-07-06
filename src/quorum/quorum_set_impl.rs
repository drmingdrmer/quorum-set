use std::collections::BTreeSet;

use crate::Node;
use crate::QuorumTree;
use crate::quorum::quorum_set::QuorumSet;

/// Impl a simple majority quorum set
impl<ID> QuorumSet for BTreeSet<ID>
where ID: PartialOrd + Ord + Clone + 'static
{
    type Id = ID;
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I: Iterator<Item = &'a ID> + Clone>(&self, ids: I) -> bool {
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

/// Impl a joint quorum set: a node set is a quorum iff it is a majority in every config.
impl<NID> QuorumSet for Vec<BTreeSet<NID>>
where NID: PartialOrd + Ord + Clone + 'static
{
    type Id = NID;
    type Iter = std::collections::btree_set::IntoIter<NID>;

    fn is_quorum<'a, I: Iterator<Item = &'a NID> + Clone>(&self, ids: I) -> bool {
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

/// Impl a hierarchical quorum set.
impl<ID> QuorumSet for QuorumTree<ID>
where ID: Ord + Clone + 'static
{
    type Id = ID;
    type Iter = std::collections::btree_set::IntoIter<ID>;

    fn is_quorum<'a, I: Iterator<Item = &'a ID> + Clone>(&self, ids: I) -> bool {
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
