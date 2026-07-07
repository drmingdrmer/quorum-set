use std::collections::BTreeSet;

use maplit::btreeset;

use crate::quorum::QuorumBridge;
use crate::quorum::QuorumIntersection;

#[test]
fn test_intersects_with() -> anyhow::Result<()> {
    let s123 = || btreeset! {1,2,3};
    let s345 = || btreeset! {3,4,5};
    let s789 = || btreeset! {7,8,9};

    let j123 = vec![s123()];
    let j345 = vec![s345()];
    let j123_345 = vec![s123(), s345()];
    let j345_789 = vec![s345(), s789()];

    // `intersects_with` returns `true` iff the two joint quorum sets share a config.
    assert!(j123.intersects_with(&j123));
    assert!(!j123.intersects_with(&j345));
    assert!(j123.intersects_with(&j123_345));
    assert!(!j123.intersects_with(&j345_789));

    assert!(!j345.intersects_with(&j123));
    assert!(j345.intersects_with(&j345));
    assert!(j345.intersects_with(&j123_345));
    assert!(j345.intersects_with(&j345_789));

    assert!(j123_345.intersects_with(&j123));
    assert!(j123_345.intersects_with(&j345));
    assert!(j123_345.intersects_with(&j123_345));
    assert!(j123_345.intersects_with(&j345_789));

    assert!(!j345_789.intersects_with(&j123));
    assert!(j345_789.intersects_with(&j345));
    assert!(j345_789.intersects_with(&j123_345));
    assert!(j345_789.intersects_with(&j345_789));

    Ok(())
}

#[test]
fn test_intersects_with_empty_joint() -> anyhow::Result<()> {
    let empty: Vec<BTreeSet<u64>> = vec![];
    let j123 = vec![btreeset! {1,2,3}];

    // An empty joint accepts every candidate set, including the empty set.
    // The empty set intersects nothing, so an empty joint has quorum
    // intersection with no joint config, not even itself.
    assert!(!empty.intersects_with(&j123));
    assert!(!j123.intersects_with(&empty));
    assert!(!empty.intersects_with(&empty));

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
