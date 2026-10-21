use std::collections::BTreeSet;

use maplit::btreeset;

use crate::quorum::QuorumBridge;
use crate::quorum::quorum_intersection_test::all_configs;
use crate::quorum::quorum_intersection_test::all_joints;
use crate::quorum::verify_intersection;

#[test]
fn test_bridge_to_preserves_intersection() {
    let joints = all_joints();
    let goals = all_configs();

    for current in &joints {
        for goal in &goals {
            let intermediate = current.bridge_to(goal.clone());
            assert!(
                verify_intersection(current, &intermediate),
                "{current:?} must intersect its bridge {intermediate:?}"
            );
            assert!(
                verify_intersection(&intermediate, goal),
                "bridge {intermediate:?} must intersect its goal {goal:?}"
            );
        }
    }
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

    // Every quorum of `{1,2,3}` intersects every quorum of `{1,2,3,4}`, so the bridge is the
    // target alone.
    let grown = btreeset! {1,2,3,4};
    let bridge = j1.bridge_to(grown.clone());
    assert_eq!(vec![grown], bridge);

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
