//! Quorum rule checks shared by unit tests.

use std::collections::BTreeSet;
use std::fmt;

use crate::quorum::QuorumSet;
use crate::quorum::threshold_quorum_set_test::all_subsets;

/// Returns the sets in `subsets` that `quorum_set` accepts, in their order.
pub(super) fn accepted<Q>(quorum_set: &Q, subsets: &[BTreeSet<u64>]) -> Vec<BTreeSet<u64>>
where Q: QuorumSet<Id = u64> {
    let mut accepted = vec![];
    for subset in subsets {
        let is_quorum = quorum_set.is_quorum(subset.iter());
        if is_quorum {
            accepted.push(subset.clone());
        }
    }
    accepted
}

/// Returns the sets in `subsets` that hold one of `quorums`.
pub(super) fn holding_one_of(
    quorums: &[BTreeSet<u64>],
    subsets: &[BTreeSet<u64>],
) -> Vec<BTreeSet<u64>> {
    let mut holding = vec![];
    for subset in subsets {
        let holds = quorums.iter().any(|quorum| quorum.is_subset(subset));
        if holds {
            holding.push(subset.clone());
        }
    }
    holding
}

/// Asserts the [`QuorumSet`] contract on every subset of the IDs of `quorum_set`: the empty set
/// is no quorum, repeated IDs and an unknown ID change no result, and adding an ID to a quorum
/// keeps it a quorum.
pub(super) fn assert_quorum_set_contract<Q>(quorum_set: &Q)
where Q: QuorumSet<Id = u64> + fmt::Debug {
    let ids: BTreeSet<u64> = quorum_set.ids().collect();
    let max_id = ids.last().copied();
    let unknown = max_id.unwrap_or(0) + 1;

    let accepts_empty = quorum_set.is_quorum([].iter());
    assert!(!accepts_empty, "{quorum_set:?} accepts the empty set");

    for subset in all_subsets(&ids) {
        let accepts = quorum_set.is_quorum(subset.iter());

        let mut repeated: Vec<u64> = subset.iter().copied().collect();
        repeated.extend(&subset);
        let accepts_repeated = quorum_set.is_quorum(repeated.iter());
        assert_eq!(accepts, accepts_repeated, "{quorum_set:?} on {repeated:?}");

        let mut with_unknown = subset.clone();
        with_unknown.insert(unknown);
        let accepts_with_unknown = quorum_set.is_quorum(with_unknown.iter());
        assert_eq!(
            accepts, accepts_with_unknown,
            "{quorum_set:?} on {with_unknown:?}"
        );

        if !accepts {
            continue;
        }
        for id in &ids {
            let mut larger = subset.clone();
            larger.insert(*id);
            let accepts_larger = quorum_set.is_quorum(larger.iter());
            assert!(
                accepts_larger,
                "{quorum_set:?} accepts {subset:?} but not {larger:?}"
            );
        }
    }
}
