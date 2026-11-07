//! Quorum rule checks shared by unit tests.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use crate::quorum::Group;
use crate::quorum::QuorumSet;
use crate::quorum::threshold_quorum_set_test::all_subsets;

/// Returns every sequence of `len` distinct IDs from `ids`.
pub(super) fn arrangements(ids: &[u64], len: usize) -> Vec<Vec<u64>> {
    if len == 0 {
        return vec![vec![]];
    }

    let mut sequences = vec![];
    for (i, id) in ids.iter().enumerate() {
        let mut rest = ids.to_vec();
        rest.remove(i);
        for tail in arrangements(&rest, len - 1) {
            let mut sequence = vec![*id];
            sequence.extend(tail);
            sequences.push(sequence);
        }
    }
    sequences
}

/// Returns every grid of at most 4 cells that holds distinct IDs of 1 to 4.
pub(super) fn small_grids() -> Vec<Vec<Vec<u64>>> {
    let shapes = [
        (1, 1),
        (1, 2),
        (2, 1),
        (1, 3),
        (3, 1),
        (1, 4),
        (4, 1),
        (2, 2),
    ];
    let mut grids = vec![];
    for (row_count, column_count) in shapes {
        for cells in arrangements(&[1, 2, 3, 4], row_count * column_count) {
            let rows = cells.chunks(column_count);
            let grid: Vec<Vec<u64>> = rows.map(|row| row.to_vec()).collect();
            grids.push(grid);
        }
    }
    grids
}

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

/// Returns the sets in `subsets` that share an ID with each of `sets`.
pub(super) fn meeting_each_of(
    sets: &[BTreeSet<u64>],
    subsets: &[BTreeSet<u64>],
) -> Vec<BTreeSet<u64>> {
    let mut meeting = vec![];
    for subset in subsets {
        let meets_each = sets.iter().all(|set| !set.is_disjoint(subset));
        if meets_each {
            meeting.push(subset.clone());
        }
    }
    meeting
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

/// Returns every group over the tree of `layout`: each quorum size from 1 to the member count, in
/// `layout` and in each group below it.
pub(super) fn all_group_sizes(layout: &Group<u64>) -> Vec<Group<u64>> {
    let mut all_children = vec![BTreeMap::new()];
    for (name, child) in &layout.groups {
        let mut extended = vec![];
        for children in &all_children {
            for sized in all_group_sizes(child) {
                let mut children = children.clone();
                children.insert(name.clone(), sized);
                extended.push(children);
            }
        }
        all_children = extended;
    }

    let member_count = layout.ids.len() + layout.groups.len();
    let mut groups = vec![];
    for quorum_size in 1..=member_count as u64 {
        for children in &all_children {
            let ids = layout.ids.clone();
            let groups_below = children.clone();
            groups.push(Group {
                quorum_size,
                ids,
                groups: groups_below,
            });
        }
    }
    groups
}
