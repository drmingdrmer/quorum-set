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

#[test]
fn test_bridge_to_joint_preserves_intersection() {
    let mut joints = all_joints();
    // Joints over IDs 0 to 3 keep this check short: pairs of joints over all 5 IDs take about 45
    // seconds.
    joints.retain(|joint| joint.iter().all(|config| !config.contains(&4)));

    for current in &joints {
        for goal in &joints {
            let intermediate = current.bridge_to(goal.clone());
            let current_to_bridge = verify_intersection(current, &intermediate);
            assert!(
                current_to_bridge,
                "{current:?} must intersect its bridge {intermediate:?}"
            );
            let bridge_to_goal = verify_intersection(&intermediate, goal);
            assert!(
                bridge_to_goal,
                "bridge {intermediate:?} must intersect its goal {goal:?}"
            );
        }
    }
}

#[test]
fn test_bridge_to_joint_matches_config_target() {
    let joints = all_joints();
    let configs = all_configs();

    for current in &joints {
        for config in &configs {
            let to_joint = current.bridge_to(vec![config.clone()]);
            let to_config = current.bridge_to(config.clone());
            assert_eq!(to_config, to_joint, "{current:?} bridges to {config:?}");
        }
    }
}

#[test]
fn test_bridge_to_joint() -> anyhow::Result<()> {
    let s1 = || btreeset! {1,2,3};
    let s2 = || btreeset! {3,4,5};
    let s3 = || btreeset! {5,6,7};
    let s4 = || btreeset! {7,8,9};

    let j12 = vec![s1(), s2()];
    let j23 = vec![s2(), s3()];
    let j34 = vec![s3(), s4()];

    // `{3,4,5}` is in both joints, so the bridge is the target alone.
    let bridge = j12.bridge_to(j23.clone());
    assert_eq!(j23, bridge);

    // No config of `j12` intersects a config of `j34`: joint of the last config of `j12` and the
    // configs of `j34`.
    let bridge = j12.bridge_to(j34);
    assert_eq!(vec![s2(), s3(), s4()], bridge);

    Ok(())
}
