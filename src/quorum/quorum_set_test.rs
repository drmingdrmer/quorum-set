use std::collections::BTreeSet;
use std::sync::Arc;

use maplit::btreeset;

use crate::Node;
use crate::QuorumTree;
use crate::quorum::QuorumSet;

fn id(i: u64) -> Node<u64> {
    Node::Id(i)
}

#[test]
fn test_simple_quorum_set_impl() -> anyhow::Result<()> {
    // BTreeSet as majority quorum set
    {
        let m12345 = btreeset! {1,2,3,4,5};

        assert!(!m12345.is_quorum([0].iter()));
        assert!(!m12345.is_quorum([0, 1, 2].iter()));
        assert!(!m12345.is_quorum([6, 7, 8].iter()));
        assert!(m12345.is_quorum([1, 2, 3].iter()));
        assert!(m12345.is_quorum([3, 4, 5].iter()));
        assert!(m12345.is_quorum([1, 3, 4, 5].iter()));
    }

    Ok(())
}

#[test]
fn test_simple_quorum_set_dedupes_candidate_ids() -> anyhow::Result<()> {
    let m12345 = btreeset! {1,2,3,4,5};

    assert!(!m12345.is_quorum([1, 1, 1].iter()));
    assert!(!m12345.is_quorum([1, 2, 2].iter()));
    assert!(m12345.is_quorum([1, 2, 3, 3].iter()));

    Ok(())
}

#[test]
fn test_empty_simple_quorum_set() -> anyhow::Result<()> {
    let empty: BTreeSet<u64> = btreeset! {};

    // An empty `BTreeSet` accepts no candidate set, not even the empty one.
    // This is the documented contrast to an empty joint config, which accepts
    // every candidate set.
    assert!(!empty.is_quorum(std::iter::empty()));
    assert!(!empty.is_quorum([1, 2].iter()));
    assert_eq!(btreeset! {}, empty.ids().collect());

    Ok(())
}

#[test]
fn test_even_sized_simple_quorum_set() -> anyhow::Result<()> {
    // 4 voters: a majority requires 3, exactly half is not a quorum.
    let m1234 = btreeset! {1,2,3,4};

    assert!(!m1234.is_quorum([1, 2].iter()));
    assert!(!m1234.is_quorum([3, 4].iter()));
    assert!(m1234.is_quorum([1, 2, 3].iter()));
    assert!(m1234.is_quorum([2, 3, 4].iter()));

    Ok(())
}

#[test]
fn test_joint_quorum_set_impl() -> anyhow::Result<()> {
    // Vec<BTreeSet> as majority quorum set
    {
        let qs = vec![btreeset! {1,2,3,4,5}];

        assert!(!qs.is_quorum([0].iter()));
        assert!(!qs.is_quorum([0, 1, 2].iter()));
        assert!(!qs.is_quorum([6, 7, 8].iter()));
        assert!(qs.is_quorum([1, 2, 3].iter()));
        assert!(qs.is_quorum([3, 4, 5].iter()));
        assert!(qs.is_quorum([1, 3, 4, 5].iter()));
    }

    // Vec<BTreeSet, BTreeSet> as joint-of-majority quorum set
    {
        let qs = vec![btreeset! {1,2,3,4,5}, btreeset! {6,7,8}];

        assert!(!qs.is_quorum([0].iter()));
        assert!(!qs.is_quorum([0, 1, 2].iter()));
        assert!(!qs.is_quorum([6, 7, 8].iter()));
        assert!(!qs.is_quorum([1, 2, 3].iter()));
        assert!(qs.is_quorum([1, 2, 3, 6, 7].iter()));
        assert!(qs.is_quorum([1, 2, 3, 4, 7, 8].iter()));
    }

    Ok(())
}

#[test]
fn test_joint_quorum_set_dedupes_candidate_ids() -> anyhow::Result<()> {
    let qs = vec![btreeset! {1,2,3,4,5}, btreeset! {6,7,8}];

    assert!(!qs.is_quorum([1, 2, 3, 6, 6].iter()));
    assert!(qs.is_quorum([1, 2, 3, 6, 7, 7].iter()));

    Ok(())
}

#[test]
fn test_empty_joint_quorum_set() -> anyhow::Result<()> {
    let qs: Vec<BTreeSet<u64>> = vec![];

    assert!(qs.is_quorum([1].iter()));
    assert_eq!(btreeset! {}, qs.ids().collect());

    Ok(())
}

#[test]
fn test_joint_quorum_set_with_empty_member_config() -> anyhow::Result<()> {
    // An empty member config accepts no candidate set, so the joint config
    // rejects everything, even when every other member config is satisfied.
    // Only an empty `Vec` (no member config at all) is vacuously satisfied.
    let qs = vec![btreeset! {}, btreeset! {1,2,3}];

    assert!(!qs.is_quorum(std::iter::empty()));
    assert!(!qs.is_quorum([1, 2, 3].iter()));
    assert_eq!(btreeset! {1,2,3}, qs.ids().collect());

    let only_empty: Vec<BTreeSet<u64>> = vec![btreeset! {}];
    assert!(!only_empty.is_quorum([1].iter()));

    Ok(())
}

#[test]
fn test_ids() -> anyhow::Result<()> {
    {
        let m12345 = btreeset! {1,2,3,4,5};
        assert_eq!(btreeset! {1,2,3,4,5}, m12345.ids().collect());
    }

    {
        let qs = vec![btreeset! {1,2,3,4,5}, btreeset! {4,5,6,7,8}];
        assert_eq!(btreeset! {1,2,3,4,5,6,7,8}, qs.ids().collect());
    }

    Ok(())
}

#[test]
fn test_arc_quorum_set_impl() -> anyhow::Result<()> {
    let qset = Arc::new(btreeset! {1,2,3});

    assert_eq!(btreeset! {1,2,3}, qset.ids().collect());
    assert!(!qset.is_quorum([1].iter()));
    assert!(qset.is_quorum([1, 2].iter()));

    Ok(())
}

#[test]
fn test_tree_impl() -> anyhow::Result<()> {
    let group_a = QuorumTree::new(2, [id(1), id(2), id(3)])?;
    let group_b = QuorumTree::new(2, [id(3), id(4), id(5)])?;
    let qset = QuorumTree::new(2, [Node::Subtree(group_a), Node::Subtree(group_b)])?;

    assert_eq!(btreeset! {1,2,3,4,5}, qset.ids().collect());

    assert!(!qset.is_quorum([1, 2, 3].iter()));
    assert!(!qset.is_quorum([1, 3].iter()));
    assert!(qset.is_quorum([1, 3, 5].iter()));
    assert!(qset.is_quorum([1, 2, 3, 4].iter()));
    assert!(qset.is_quorum([1, 3, 4, 5].iter()));

    Ok(())
}

#[test]
fn test_tree_ids_dedupe_direct_and_nested_ids() -> anyhow::Result<()> {
    let subtree = QuorumTree::new(2, [id(1), id(2), id(3)])?;
    let qset = QuorumTree::new(1, [Node::Id(1), Node::Subtree(subtree)])?;

    assert_eq!(btreeset! {1,2,3}, qset.ids().collect());

    Ok(())
}

#[test]
fn test_tree_dedupes_candidate_ids() -> anyhow::Result<()> {
    let group_a = QuorumTree::new(2, [id(1), id(2), id(3)])?;
    let group_b = QuorumTree::new(2, [id(4), id(5), id(6)])?;
    let qset = QuorumTree::new(2, [Node::Subtree(group_a), Node::Subtree(group_b)])?;

    assert!(!qset.is_quorum([1, 1, 2, 4].iter()));
    assert!(qset.is_quorum([1, 1, 2, 4, 5].iter()));

    Ok(())
}
