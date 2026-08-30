use std::collections::BTreeSet;

use maplit::btreeset;

use crate::Node;
use crate::QuorumTree;
use crate::quorum::QuorumBridge;
use crate::quorum::QuorumIntersection;
use crate::quorum::verify_intersection;

#[test]
fn test_intersects_with() -> anyhow::Result<()> {
    let s123 = || btreeset! {1,2,3};
    let s345 = || btreeset! {3,4,5};
    let s789 = || btreeset! {7,8,9};

    let j123 = vec![s123()];
    let j345 = vec![s345()];
    let j123_345 = vec![s123(), s345()];
    let j345_789 = vec![s345(), s789()];

    // `intersects_with` returns `Some(true)` iff the two joint quorum sets share a
    // config, and `None` otherwise: no shared config leaves the relation unproven.
    assert_eq!(Some(true), j123.intersects_with(&j123));
    assert_eq!(None, j123.intersects_with(&j345));
    assert_eq!(Some(true), j123.intersects_with(&j123_345));
    assert_eq!(None, j123.intersects_with(&j345_789));

    assert_eq!(None, j345.intersects_with(&j123));
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

    // An empty joint accepts every candidate set, including the empty set.
    // The empty set intersects nothing, so an empty joint has quorum
    // intersection with no joint config, not even itself.
    assert_eq!(Some(false), empty.intersects_with(&j123));
    assert_eq!(Some(false), j123.intersects_with(&empty));
    assert_eq!(Some(false), empty.intersects_with(&empty));

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
    let j123 = vec![btreeset! {1, 2, 3}];
    let j12 = vec![btreeset! {1, 2}];

    // Every majority of {1,2} intersects every majority of {1,2,3}, but the
    // shared-config heuristic cannot prove it, so it answers `None`.
    assert!(verify_intersection(&j123, &j12));
    assert_eq!(None, j123.intersects_with(&j12));
}

#[test]
fn test_intersects_with_agrees_with_verify_intersection() {
    let joints: Vec<Vec<BTreeSet<u64>>> = vec![
        vec![],
        vec![btreeset! {1, 2, 3}],
        vec![btreeset! {3, 4, 5}],
        vec![btreeset! {1, 2, 3}, btreeset! {3, 4, 5}],
        vec![btreeset! {3, 4, 5}, btreeset! {7, 8, 9}],
    ];

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
fn test_bridge_to() -> anyhow::Result<()> {
    let s1 = || btreeset! {1,2,3};
    let s2 = || btreeset! {3,4,5};
    let s3 = || btreeset! {7,8,9};

    let j1 = vec![s1()];
    let j2 = vec![s2()];
    let j12 = vec![s1(), s2()];
    let j23 = vec![s2(), s3()];

    assert_eq!(j1, j1.bridge_to(s1()));
    assert_eq!(j12, j1.bridge_to(s2()));
    assert_eq!(j1, j12.bridge_to(s1()));
    assert_eq!(j2, j12.bridge_to(s2()));
    assert_eq!(j23, j12.bridge_to(s3()));

    Ok(())
}

#[test]
fn test_bridge_to_multi_config_joint() -> anyhow::Result<()> {
    let s1 = || btreeset! {1,2,3};
    let s2 = || btreeset! {3,4,5};
    let s3 = || btreeset! {5,6,7};
    let s4 = || btreeset! {7,8,9};

    let joint = vec![s1(), s2(), s3()];

    // Sharing any member config, not only the last one, keeps just the target.
    assert_eq!(vec![s2()], joint.bridge_to(s2()));
    // No shared config: joint of the last member config and the target.
    assert_eq!(vec![s3(), s4()], joint.bridge_to(s4()));

    Ok(())
}

#[test]
fn test_bridge_to_from_empty_joint_config() -> anyhow::Result<()> {
    let joint: Vec<BTreeSet<u64>> = vec![];
    let other = btreeset! {1,2,3};

    assert_eq!(vec![other.clone()], joint.bridge_to(other));

    Ok(())
}
