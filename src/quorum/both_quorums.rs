use std::collections::BTreeSet;
use std::collections::btree_set;

use crate::quorum::QuorumSet;
#[cfg(doc)]
use crate::verify_intersection;

/// A quorum rule that accepts an ID set only when both a read rule and a write rule accept it.
///
/// Two read quorums may be disjoint, and so may two write quorums. Every set this rule accepts is
/// both a read quorum and a write quorum. So when every read quorum intersects every write quorum,
/// any two accepted sets intersect; [`verify_intersection`] checks that condition. A consensus
/// election can use this rule, so that two candidates cannot both win in one term.
///
/// [`ids()`](QuorumSet::ids) returns the union of the read and write IDs, each ID once.
#[derive(Clone, Debug)]
pub struct BothQuorums<R, W> {
    read: R,
    write: W,
}

impl<R, W> BothQuorums<R, W> {
    /// Builds a rule that accepts an ID set only when both `read` and `write` accept it.
    pub fn new(read: R, write: W) -> Self {
        Self { read, write }
    }
}

impl<R, W> QuorumSet for BothQuorums<R, W>
where
    R: QuorumSet,
    W: QuorumSet<Id = R::Id>,
    R::Id: Ord,
{
    type Id = R::Id;
    type Iter = btree_set::IntoIter<R::Id>;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        Self::Id: 'a,
        I: Iterator<Item = &'a Self::Id> + Clone,
    {
        let is_read_quorum = self.read.is_quorum(ids.clone());
        is_read_quorum && self.write.is_quorum(ids)
    }

    fn ids(&self) -> Self::Iter {
        let mut ids = BTreeSet::new();
        ids.extend(self.read.ids());
        ids.extend(self.write.ids());
        ids.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use maplit::btreeset;

    use super::BothQuorums;
    use crate::Node;
    use crate::QuorumTree;
    use crate::quorum::QuorumSet;
    use crate::quorum::verify_intersection;

    /// Nodes form a 2x2 grid, `1 2` over `3 4`. A read quorum holds a row and a write quorum holds
    /// a column, so the read quorums `{1,2}` and `{3,4}` are disjoint.
    #[test]
    fn test_grid_rows_and_columns() -> anyhow::Result<()> {
        let read = QuorumTree::new(1, [
            Node::Subtree(QuorumTree::new(2, [Node::Id(1), Node::Id(2)])?),
            Node::Subtree(QuorumTree::new(2, [Node::Id(3), Node::Id(4)])?),
        ])?;
        let write = QuorumTree::new(1, [
            Node::Subtree(QuorumTree::new(2, [Node::Id(1), Node::Id(3)])?),
            Node::Subtree(QuorumTree::new(2, [Node::Id(2), Node::Id(4)])?),
        ])?;
        assert!(!verify_intersection(&read, &read));
        assert!(verify_intersection(&read, &write));

        let both = BothQuorums::new(read, write);

        let mut accepted = vec![];
        for mask in 0..16_u64 {
            let ids = (1..=4).filter(|id| mask & (1 << (id - 1)) != 0);
            let ids = ids.collect::<BTreeSet<u64>>();
            let is_quorum = both.is_quorum(ids.iter());
            if is_quorum {
                accepted.push(ids);
            }
        }
        let want = vec![
            btreeset! {1,2,3},
            btreeset! {1,2,4},
            btreeset! {1,3,4},
            btreeset! {2,3,4},
            btreeset! {1,2,3,4},
        ];
        assert_eq!(want, accepted);

        assert!(verify_intersection(&both, &both));

        Ok(())
    }

    /// The read and write rules may be different types over different IDs.
    #[test]
    fn test_ids_are_union_of_read_and_write_ids() -> anyhow::Result<()> {
        let read = btreeset! {1u64, 2, 3};
        let write = QuorumTree::new(2, [Node::Id(3), Node::Id(4), Node::Id(5)])?;
        let both = BothQuorums::new(read, write);

        let ids = both.ids().collect::<Vec<_>>();
        assert_eq!(vec![1, 2, 3, 4, 5], ids);

        Ok(())
    }
}
