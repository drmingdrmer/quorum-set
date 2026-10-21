use std::collections::BTreeSet;

use maplit::btreeset;

use crate::Node;
use crate::QuorumTree;
use crate::quorum::QuorumIntersection;
use crate::quorum::verify_intersection;

const UNIVERSE_SIZE: u64 = 5;

pub(super) fn all_configs() -> Vec<BTreeSet<u64>> {
    (1..1u64 << UNIVERSE_SIZE)
        .map(|mask| (0..UNIVERSE_SIZE).filter(|id| mask & (1 << id) != 0).collect())
        .collect()
}

pub(super) fn all_joints() -> Vec<Vec<BTreeSet<u64>>> {
    let configs = all_configs();
    let singles = configs.iter().map(|config| vec![config.clone()]);
    let pairs =
        configs.iter().flat_map(|a| configs.iter().map(move |b| vec![a.clone(), b.clone()]));
    singles.chain(pairs).collect()
}

#[test]
fn test_intersects_with() -> anyhow::Result<()> {
    let s123 = || btreeset! {1,2,3};
    let s345 = || btreeset! {3,4,5};
    let s789 = || btreeset! {7,8,9};

    let j123 = vec![s123()];
    let j345 = vec![s345()];
    let j123_345 = vec![s123(), s345()];
    let j345_789 = vec![s345(), s789()];

    // Two uniform joints get the exact answer: {1,2} and {4,5} are disjoint
    // majorities of {1,2,3} and {3,4,5}. Other joints answer `Some(true)` when
    // some config pair intersects, and `None` when no pair does.
    assert_eq!(Some(true), j123.intersects_with(&j123));
    assert_eq!(Some(false), j123.intersects_with(&j345));
    assert_eq!(Some(true), j123.intersects_with(&j123_345));
    assert_eq!(None, j123.intersects_with(&j345_789));

    assert_eq!(Some(false), j345.intersects_with(&j123));
    assert_eq!(Some(true), j345.intersects_with(&j345));
    assert_eq!(Some(true), j345.intersects_with(&j123_345));
    assert_eq!(Some(true), j345.intersects_with(&j345_789));

    assert_eq!(Some(true), j123_345.intersects_with(&j123));
    assert_eq!(Some(true), j123_345.intersects_with(&j345));
    assert_eq!(Some(true), j123_345.intersects_with(&j123_345));
    assert_eq!(Some(true), j123_345.intersects_with(&j345_789));

    assert_eq!(None, j345_789.intersects_with(&j123));
    assert_eq!(Some(true), j345_789.intersects_with(&j345));
    assert_eq!(Some(true), j345_789.intersects_with(&j123_345));
    assert_eq!(Some(true), j345_789.intersects_with(&j345_789));

    Ok(())
}

#[test]
fn test_intersects_with_empty_joint() -> anyhow::Result<()> {
    let empty: Vec<BTreeSet<u64>> = vec![];
    let j123 = vec![btreeset! {1,2,3}];
    let no_quorum = vec![BTreeSet::new(), btreeset! {1,2,3}];

    // An empty joint accepts every candidate set, including the empty set.
    // The empty set intersects nothing, so an empty joint has quorum
    // intersection only with a joint that has no quorum: one that holds an
    // empty config.
    assert_eq!(Some(false), empty.intersects_with(&j123));
    assert_eq!(Some(false), j123.intersects_with(&empty));
    assert_eq!(Some(false), empty.intersects_with(&empty));

    let empty_with_no_quorum = empty.intersects_with(&no_quorum);
    assert_eq!(Some(true), empty_with_no_quorum);
    let no_quorum_with_empty = no_quorum.intersects_with(&empty);
    assert_eq!(Some(true), no_quorum_with_empty);

    Ok(())
}

#[test]
fn test_verify_intersection() {
    let s123 = btreeset! {1, 2, 3};
    let s12 = btreeset! {1, 2};
    let s45 = btreeset! {4, 5};

    // Majorities of one voter set always intersect each other.
    assert!(verify_intersection(&s123, &s123));
    // The only majority of {1,2} is {1,2}, which intersects every majority of {1,2,3}.
    assert!(verify_intersection(&s12, &s123));
    assert!(verify_intersection(&s123, &s12));
    // Majorities of disjoint voter sets never intersect.
    assert!(!verify_intersection(&s123, &s45));
}

#[test]
fn test_verify_intersection_empty_joint() {
    let empty: Vec<BTreeSet<u64>> = vec![];
    let j123 = vec![btreeset! {1, 2, 3}];

    // An empty joint accepts the empty set as a quorum, and the empty set
    // intersects nothing.
    assert!(!verify_intersection(&empty, &j123));
    assert!(!verify_intersection(&empty, &empty));
}

#[test]
fn test_verify_intersection_exact_where_intersects_with_is_conservative() {
    let j123_145 = vec![btreeset! {1, 2, 3}, btreeset! {1, 4, 5}];
    let j2345 = vec![btreeset! {2, 3, 4, 5}];

    // Every quorum of the joint holds 2 of {2,3,4,5}, and every majority of
    // {2,3,4,5} holds 3 of them, so the quorums intersect. No config pair
    // proves it: {1,2} misses {3,4,5}, and {1,4} misses {2,3,5}.
    let exact = verify_intersection(&j123_145, &j2345);
    assert!(exact);
    let intersects = j123_145.intersects_with(&j2345);
    assert_eq!(None, intersects);
}

#[test]
fn test_intersects_with_agrees_with_verify_intersection() {
    let mut joints: Vec<Vec<BTreeSet<u64>>> = vec![
        vec![],
        vec![BTreeSet::new()],
        vec![btreeset! {3, 4, 5}],
        vec![btreeset! {1, 2, 3}, btreeset! {3, 4, 5}],
        vec![btreeset! {3, 4, 5}, btreeset! {7, 8, 9}],
    ];
    joints.extend(all_joints());

    // `intersects_with` may answer `None`, but every `Some` answer must match the
    // exact relation that `verify_intersection` computes.
    for a in &joints {
        for b in &joints {
            let Some(intersects) = a.intersects_with(b) else {
                continue;
            };
            let exact = verify_intersection(a, b);
            assert_eq!(exact, intersects, "a: {a:?}, b: {b:?}");
        }
    }
}

#[test]
fn test_verify_intersection_tree_read_write() -> anyhow::Result<()> {
    // Write quorums: majorities of {1,2,3}.
    let write = btreeset! {1u64, 2, 3};

    // Read quorums: {2,3} or {8,9}. Every write quorum intersects *some* read
    // quorum, but a read via {8,9} misses every write quorum, so the
    // universal relation does not hold.
    let read = QuorumTree::new(1, [
        Node::Subtree(QuorumTree::new(2, [Node::Id(2), Node::Id(3)])?),
        Node::Subtree(QuorumTree::new(2, [Node::Id(8), Node::Id(9)])?),
    ])?;
    assert!(!verify_intersection(&read, &write));

    // Dropping the {8,9} branch restores intersection.
    let read = QuorumTree::new(2, [Node::Id(2), Node::Id(3)])?;
    assert!(verify_intersection(&read, &write));

    Ok(())
}

#[test]
fn test_btree_set_intersects_with() {
    let s123 = btreeset! {1, 2, 3};
    let s124 = btreeset! {1, 2, 4};
    let s1234 = btreeset! {1, 2, 3, 4};
    let s1235 = btreeset! {1, 2, 3, 5};

    // Adding a voter: every majority of {1,2,3,4} holds 2 of {1,2,3}.
    let added = s123.intersects_with(&s1234);
    assert_eq!(Some(true), added);

    // Replacing one of four voters: every majority of either set holds 2 of {1,2,3}.
    let replaced_of_four = s1234.intersects_with(&s1235);
    assert_eq!(Some(true), replaced_of_four);

    // Replacing one of three voters: {1,3} and {2,4} are disjoint majorities.
    let replaced_of_three = s123.intersects_with(&s124);
    assert_eq!(Some(false), replaced_of_three);
}

#[test]
fn test_btree_set_intersects_with_agrees_with_verify_intersection() {
    let mut configs = all_configs();
    configs.push(BTreeSet::new());

    // The flat check always answers, and every answer matches the exact relation.
    for a in &configs {
        for b in &configs {
            let intersects = a.intersects_with(b);
            let exact = verify_intersection(a, b);
            assert_eq!(Some(exact), intersects, "a: {a:?}, b: {b:?}");
        }
    }
}

#[test]
fn test_joint_intersects_with_config() {
    let s123 = btreeset! {1, 2, 3};
    let s124 = btreeset! {1, 2, 4};
    let s1234 = btreeset! {1, 2, 3, 4};
    let s2345 = btreeset! {2, 3, 4, 5};
    let empty_config = BTreeSet::new();

    let j123 = vec![s123.clone()];
    let j123_145 = vec![s123.clone(), btreeset! {1, 4, 5}];
    let empty_joint: Vec<BTreeSet<u64>> = vec![];

    // A uniform joint gets the exact answer of the flat check.
    let added = j123.intersects_with(&s1234);
    assert_eq!(Some(true), added);
    let replaced = j123.intersects_with(&s124);
    assert_eq!(Some(false), replaced);

    // Another joint answers `Some(true)` when one of its configs intersects the flat config, and
    // `None` when none does, although every quorum of `j123_145` intersects every majority of
    // {2,3,4,5}.
    let one_config_intersects = j123_145.intersects_with(&s1234);
    assert_eq!(Some(true), one_config_intersects);
    let no_config_intersects = j123_145.intersects_with(&s2345);
    assert_eq!(None, no_config_intersects);

    // The empty set is a quorum of an empty joint. It misses every majority of {1,2,3}, and an
    // empty config has no quorum to miss.
    let empty_joint_with_config = empty_joint.intersects_with(&s123);
    assert_eq!(Some(false), empty_joint_with_config);
    let empty_joint_with_empty_config = empty_joint.intersects_with(&empty_config);
    assert_eq!(Some(true), empty_joint_with_empty_config);

    // A flat config gets the same answer from the other side.
    let config_with_joint = s124.intersects_with(&j123);
    assert_eq!(Some(false), config_with_joint);
}

#[test]
fn test_joint_intersects_with_config_agrees_with_verify_intersection() {
    let mut joints = all_joints();
    joints.push(vec![]);
    let mut configs = all_configs();
    configs.push(BTreeSet::new());

    // Both sides give the same answer. A joint of at most one config always gets the exact answer,
    // and every `Some` answer for a larger joint matches the exact relation.
    for joint in &joints {
        for config in &configs {
            let intersects = joint.intersects_with(config);
            let reversed = config.intersects_with(joint);
            assert_eq!(intersects, reversed, "joint: {joint:?}, config: {config:?}");

            let exact = verify_intersection(joint, config);
            let at_most_one_config = joint.len() <= 1;
            if at_most_one_config {
                assert_eq!(
                    Some(exact),
                    intersects,
                    "joint: {joint:?}, config: {config:?}"
                );
                continue;
            }

            let Some(intersects) = intersects else {
                continue;
            };
            assert_eq!(exact, intersects, "joint: {joint:?}, config: {config:?}");
        }
    }
}
