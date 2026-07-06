use std::collections::BTreeSet;

use super::canonical_id::CanonicalId;
use crate::Node;
use crate::QuorumTreeError;

mod impl_canonical_id;
mod impl_display;

#[derive(Clone, Debug)]
pub(crate) struct QuorumTreeSpec<ID>
where ID: Ord
{
    quorum_size: u64,

    nodes: BTreeSet<Node<ID>>,
}

impl<ID> QuorumTreeSpec<ID>
where ID: Ord
{
    pub(crate) fn new(
        quorum_size: u64,
        nodes: impl IntoIterator<Item = Node<ID>>,
    ) -> Result<Self, QuorumTreeError>
    where
        ID: CanonicalId,
    {
        let mut unique = BTreeSet::new();
        for node in nodes {
            if let Some(dup) = unique.replace(node) {
                return Err(QuorumTreeError::DuplicateChild {
                    canonical_id: dup.canonical_id(),
                });
            }
        }

        if quorum_size > unique.len() as u64 {
            return Err(QuorumTreeError::UnsatisfiableQuorum {
                quorum_size,
                num_children: unique.len(),
            });
        }

        Ok(Self {
            quorum_size,
            nodes: unique,
        })
    }

    pub(crate) fn quorum_size(&self) -> u64 {
        self.quorum_size
    }

    pub(crate) fn children(&self) -> impl Iterator<Item = &Node<ID>> {
        self.nodes.iter()
    }

    pub(crate) fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: IntoIterator<Item = &'a ID> + Clone,
    {
        let required = self.quorum_size();
        if required == 0 {
            return true;
        }

        let mut count = 0;
        for node in &self.nodes {
            if node.is_selected_by(ids.clone()) {
                count += 1;
            }
            if count >= required {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::QuorumTreeSpec;
    use crate::Node;
    use crate::QuorumTree;
    use crate::QuorumTreeError;

    fn id(i: u64) -> Node<u64> {
        Node::Id(i)
    }

    fn qset(quorum_size: u64, nodes: &[u64]) -> QuorumTreeSpec<u64> {
        QuorumTreeSpec::new(quorum_size, nodes.iter().copied().map(id)).unwrap()
    }

    fn set(quorum_size: u64, nodes: &[u64]) -> Node<u64> {
        Node::Subtree(QuorumTree::new(quorum_size, nodes.iter().copied().map(id)).unwrap())
    }

    fn raft_config(nodes: &[u64]) -> Node<u64> {
        let majority = nodes.len() as u64 / 2 + 1;
        set(majority, nodes)
    }

    fn btreeset(nodes: &[u64]) -> BTreeSet<u64> {
        nodes.iter().copied().collect()
    }

    fn all_subsets(ids: &[u64]) -> Vec<Vec<u64>> {
        (0..(1 << ids.len()))
            .map(|mask| {
                ids.iter()
                    .enumerate()
                    .filter(|(i, _)| mask & (1 << i) != 0)
                    .map(|(_, id)| *id)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn is_subset(lhs: &[u64], rhs: &[u64]) -> bool {
        lhs.iter().all(|id| rhs.contains(id))
    }

    fn assert_is_quorum_monotonic(qset: &QuorumTreeSpec<u64>, ids: &[u64]) {
        let subsets = all_subsets(ids);

        for lhs in &subsets {
            if !qset.is_quorum(lhs) {
                continue;
            }

            for rhs in &subsets {
                if is_subset(lhs, rhs) {
                    assert!(qset.is_quorum(rhs), "lhs: {lhs:?}, rhs: {rhs:?}");
                }
            }
        }
    }

    fn is_majority_quorum(config: &BTreeSet<u64>, ids: &[u64]) -> bool {
        let majority = config.len() / 2 + 1;
        config.iter().filter(|id| ids.contains(id)).count() >= majority
    }

    fn assert_quorum_matches_raft_joint(
        qset: &QuorumTreeSpec<u64>,
        configs: Vec<BTreeSet<u64>>,
        ids: &[u64],
    ) {
        for selected in all_subsets(ids) {
            assert_eq!(
                configs.iter().all(|config| is_majority_quorum(config, &selected)),
                qset.is_quorum(&selected),
                "selected ids: {selected:?}"
            );
        }
    }

    #[test]
    fn test_0_nodes_quorum_0() {
        let qset = qset(0, &[]);

        assert_eq!(0, qset.quorum_size());

        assert!(qset.is_quorum(&[]));
        assert!(qset.is_quorum(&[1]));
    }

    #[test]
    fn test_0_nodes_quorum_1() {
        let err = QuorumTreeSpec::<u64>::new(1, []).unwrap_err();

        assert_eq!(
            QuorumTreeError::UnsatisfiableQuorum {
                quorum_size: 1,
                num_children: 0
            },
            err
        );
    }

    #[test]
    fn test_3_nodes_quorum_0() {
        let qset = qset(0, &[1, 2, 3]);

        assert_eq!(0, qset.quorum_size());

        assert!(qset.is_quorum(&[]));
        assert!(qset.is_quorum(&[1]));
        assert!(qset.is_quorum(&[1, 2, 3]));
    }

    #[test]
    fn test_3_nodes_quorum_1() {
        let qset = qset(1, &[1, 2, 3]);

        assert_eq!(1, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(qset.is_quorum(&[1]));
        assert!(qset.is_quorum(&[2]));
        assert!(qset.is_quorum(&[1, 2, 3]));
    }

    #[test]
    fn test_3_nodes() {
        let qset = QuorumTreeSpec::new(2, vec![id(1), id(2), id(3)]).unwrap();

        assert_eq!(2, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(!qset.is_quorum(&[1]));
        assert!(!qset.is_quorum(&[2]));
        assert!(!qset.is_quorum(&[3]));

        assert!(qset.is_quorum(&[1, 2]));
        assert!(qset.is_quorum(&[1, 3]));
        assert!(qset.is_quorum(&[2, 3]));
        assert!(qset.is_quorum(&[1, 2, 3]));
    }

    #[test]
    fn test_4_nodes() {
        let qset = QuorumTreeSpec::new(2, vec![id(1), id(2), id(3), id(4)]).unwrap();

        assert_eq!(2, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(!qset.is_quorum(&[1]));
        assert!(!qset.is_quorum(&[2]));
        assert!(!qset.is_quorum(&[3]));

        assert!(qset.is_quorum(&[1, 2]));
        assert!(qset.is_quorum(&[1, 3]));
        assert!(qset.is_quorum(&[1, 4]));
        assert!(qset.is_quorum(&[2, 3]));
        assert!(qset.is_quorum(&[2, 4]));
        assert!(qset.is_quorum(&[3, 4]));
        assert!(qset.is_quorum(&[1, 2, 3]));
        assert!(qset.is_quorum(&[1, 2, 4]));
        assert!(qset.is_quorum(&[1, 3, 4]));
        assert!(qset.is_quorum(&[2, 3, 4]));
        assert!(qset.is_quorum(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_4_nodes_quorum_4() {
        let qset = qset(4, &[1, 2, 3, 4]);

        assert_eq!(4, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(!qset.is_quorum(&[1]));
        assert!(!qset.is_quorum(&[1, 2, 3]));
        assert!(qset.is_quorum(&[1, 2, 3, 4]));
    }

    #[test]
    fn test_5_nodes_quorum_4() {
        let qset = qset(4, &[1, 2, 3, 4, 5]);

        assert_eq!(4, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(!qset.is_quorum(&[1]));
        assert!(!qset.is_quorum(&[1, 2, 3]));
        assert!(qset.is_quorum(&[1, 2, 3, 4]));
        assert!(qset.is_quorum(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_joint() {
        let read_quorum_tree =
            QuorumTreeSpec::new(1, [raft_config(&[1, 2, 3]), raft_config(&[4, 5, 6])]).unwrap();
        let write_quorum_tree =
            QuorumTreeSpec::new(2, [raft_config(&[1, 2, 3]), raft_config(&[4, 5, 6])]).unwrap();

        assert_eq!(1, read_quorum_tree.quorum_size());
        assert_eq!(2, write_quorum_tree.quorum_size());

        assert!(!read_quorum_tree.is_quorum(&[]));
        assert!(!read_quorum_tree.is_quorum(&[1]));
        assert!(!read_quorum_tree.is_quorum(&[1, 4]));

        assert!(read_quorum_tree.is_quorum(&[1, 2]));
        assert!(read_quorum_tree.is_quorum(&[4, 5]));
        assert!(read_quorum_tree.is_quorum(&[1, 2, 4, 5]));

        assert!(!write_quorum_tree.is_quorum(&[]));
        assert!(!write_quorum_tree.is_quorum(&[1]));
        assert!(!write_quorum_tree.is_quorum(&[1, 2]));
        assert!(!write_quorum_tree.is_quorum(&[4, 5]));
        assert!(!write_quorum_tree.is_quorum(&[1, 4]));

        assert!(write_quorum_tree.is_quorum(&[1, 2, 4, 5]));
        assert!(write_quorum_tree.is_quorum(&[1, 3, 4, 6]));
        assert!(write_quorum_tree.is_quorum(&[1, 2, 3, 4, 5, 6]));

        assert_quorum_matches_raft_joint(
            &write_quorum_tree,
            vec![btreeset(&[1, 2, 3]), btreeset(&[4, 5, 6])],
            &[1, 2, 3, 4, 5, 6],
        );

        let write_quorum_tree =
            QuorumTreeSpec::new(2, [raft_config(&[1, 2, 3, 4]), raft_config(&[3, 4, 5, 6])])
                .unwrap();

        assert_quorum_matches_raft_joint(
            &write_quorum_tree,
            vec![btreeset(&[1, 2, 3, 4]), btreeset(&[3, 4, 5, 6])],
            &[1, 2, 3, 4, 5, 6],
        );
    }

    #[test]
    fn test_is_quorum_is_monotonic() {
        assert_is_quorum_monotonic(&qset(0, &[1, 2, 3]), &[1, 2, 3]);
        assert_is_quorum_monotonic(&qset(2, &[1, 2, 3]), &[1, 2, 3]);
        assert_is_quorum_monotonic(&qset(4, &[1, 2, 3, 4]), &[1, 2, 3, 4]);

        let qset =
            QuorumTreeSpec::new(2, [raft_config(&[1, 2, 3]), raft_config(&[4, 5, 6])]).unwrap();
        assert_is_quorum_monotonic(&qset, &[1, 2, 3, 4, 5, 6]);

        let qset = QuorumTreeSpec::new(2, [
            set(2, &[1, 2, 3]),
            set(2, &[4, 5, 6]),
            set(2, &[7, 8, 9]),
        ])
        .unwrap();
        assert_is_quorum_monotonic(&qset, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_group() {
        let qset = QuorumTreeSpec::new(2, [
            set(2, &[1, 2, 3]),
            set(2, &[4, 5, 6]),
            set(2, &[7, 8, 9]),
        ])
        .unwrap();

        assert_eq!(2, qset.quorum_size());

        assert!(!qset.is_quorum(&[]));
        assert!(!qset.is_quorum(&[1, 2]));
        assert!(!qset.is_quorum(&[1, 4, 7]));
        assert!(!qset.is_quorum(&[1, 2, 4]));

        assert!(qset.is_quorum(&[1, 2, 4, 5]));
        assert!(qset.is_quorum(&[1, 3, 7, 9]));
        assert!(qset.is_quorum(&[4, 6, 7, 8]));
        assert!(qset.is_quorum(&[1, 2, 3, 4, 5, 6, 7, 8, 9]));
    }

    #[test]
    fn test_duplicate_subtrees_are_rejected() {
        // Canonically equal subtrees are duplicates even when built in a different node order.
        let err = QuorumTreeSpec::new(2, [set(2, &[1, 2, 3]), set(2, &[3, 2, 1])]).unwrap_err();

        assert_eq!(
            QuorumTreeError::DuplicateChild {
                canonical_id: "Subtree=2/(Id=1,Id=2,Id=3)".to_string(),
            },
            err
        );
    }
}
