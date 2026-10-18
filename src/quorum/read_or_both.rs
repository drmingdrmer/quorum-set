use std::collections::BTreeSet;
use std::collections::btree_set;

use crate::quorum::BothQuorums;
use crate::quorum::QuorumSet;

/// A quorum rule that is either a read rule alone or a [`BothQuorums`].
///
/// A consensus election picks the variant by how it orders leaders:
///
/// - With totally ordered ballots, as in Paxos, use [`Read`](Self::Read). Two leaders always differ
///   in ballot, and a voter rejects entries from a ballot smaller than one it granted. So read
///   quorums need to intersect only write quorums.
/// - When two leaders of one term are not ordered, as in Raft, use [`Both`](Self::Both). Any two
///   winning vote sets must then intersect, but two read quorums may be disjoint.
///
/// [`ids()`](QuorumSet::ids) returns the read IDs for `Read`, and the union of the read and write
/// IDs for `Both`, each ID once.
#[derive(Clone, Debug)]
pub enum ReadOrBoth<R, W> {
    /// Accepts an ID set when the read rule accepts it.
    Read(R),

    /// Accepts an ID set when both the read rule and the write rule accept it.
    Both(BothQuorums<R, W>),
}

impl<R, W> QuorumSet for ReadOrBoth<R, W>
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
        match self {
            Self::Read(read) => read.is_quorum(ids),
            Self::Both(both) => both.is_quorum(ids),
        }
    }

    fn ids(&self) -> Self::Iter {
        match self {
            Self::Read(read) => {
                let ids = read.ids().collect::<BTreeSet<_>>();
                ids.into_iter()
            }
            Self::Both(both) => both.ids(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use maplit::btreeset;

    use super::ReadOrBoth;
    use crate::Node;
    use crate::QuorumTree;
    use crate::quorum::BothQuorums;
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
        let read_only = ReadOrBoth::<_, QuorumTree<u64>>::Read(read.clone());
        let both = ReadOrBoth::Both(BothQuorums::new(read, write));

        let mut read_only_accepted = vec![];
        let mut both_accepted = vec![];
        for mask in 0..16_u64 {
            let ids = (1..=4).filter(|id| mask & (1 << (id - 1)) != 0);
            let ids = ids.collect::<BTreeSet<u64>>();
            if read_only.is_quorum(ids.iter()) {
                read_only_accepted.push(ids.clone());
            }
            if both.is_quorum(ids.iter()) {
                both_accepted.push(ids);
            }
        }

        let want = vec![
            btreeset! {1,2},
            btreeset! {1,2,3},
            btreeset! {1,2,4},
            btreeset! {3,4},
            btreeset! {1,3,4},
            btreeset! {2,3,4},
            btreeset! {1,2,3,4},
        ];
        assert_eq!(want, read_only_accepted);

        let want = vec![
            btreeset! {1,2,3},
            btreeset! {1,2,4},
            btreeset! {1,3,4},
            btreeset! {2,3,4},
            btreeset! {1,2,3,4},
        ];
        assert_eq!(want, both_accepted);

        assert!(!verify_intersection(&read_only, &read_only));
        assert!(verify_intersection(&both, &both));

        Ok(())
    }

    /// `Read` yields only the read IDs, even when the write rule holds other IDs.
    #[test]
    fn test_ids_of_each_variant() -> anyhow::Result<()> {
        let read = btreeset! {1u64, 2, 3};
        let write = QuorumTree::new(2, [Node::Id(3), Node::Id(4), Node::Id(5)])?;
        let read_only = ReadOrBoth::<_, QuorumTree<u64>>::Read(read.clone());
        let both = ReadOrBoth::Both(BothQuorums::new(read, write));

        let read_only_ids = read_only.ids().collect::<Vec<_>>();
        assert_eq!(vec![1, 2, 3], read_only_ids);

        let both_ids = both.ids().collect::<Vec<_>>();
        assert_eq!(vec![1, 2, 3, 4, 5], both_ids);

        Ok(())
    }
}
