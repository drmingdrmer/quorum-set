use std::collections::BTreeSet;

use maplit::btreeset;

use crate::quorum::QuorumBridge;
use crate::quorum::QuorumSet;
use crate::quorum::quorum_intersection_test::all_configs;
use crate::quorum::quorum_intersection_test::all_joints;

#[test]
fn test_joint_with() -> anyhow::Result<()> {
    let s1 = || btreeset! {1,2,3};
    let s2 = || btreeset! {3,4,5};
    let s3 = || btreeset! {5,6,7};
    let s4 = || btreeset! {7,8,9};

    let j12 = vec![s1(), s2()];

    let joint = j12.joint_with(s3());
    assert_eq!(vec![s1(), s2(), s3()], joint);

    let joint = j12.joint_with(vec![s3(), s4()]);
    assert_eq!(vec![s1(), s2(), s3(), s4()], joint);

    Ok(())
}

#[test]
fn test_joint_with_empty_joint() -> anyhow::Result<()> {
    let empty: Vec<BTreeSet<u64>> = vec![];
    let j1 = vec![btreeset! {1,2,3}];

    let joint = empty.joint_with(btreeset! {1,2,3});
    assert_eq!(empty, joint);

    let joint = empty.joint_with(j1.clone());
    assert_eq!(empty, joint);

    let joint = j1.joint_with(empty.clone());
    assert_eq!(empty, joint);

    Ok(())
}

#[test]
fn test_joint_with_accepts_quorums_of_both_sides() {
    let joints = all_joints();
    let configs = all_configs();
    let id_sets = all_configs();

    for current in &joints {
        for config in &configs {
            let joint = current.joint_with(config.clone());
            for ids in &id_sets {
                let in_current = current.is_quorum(ids.iter());
                let in_config = config.is_quorum(ids.iter());
                let in_both = in_current && in_config;
                let in_joint = joint.is_quorum(ids.iter());
                assert_eq!(
                    in_both, in_joint,
                    "{current:?} joint with {config:?} on {ids:?}"
                );
            }
        }
    }
}

#[test]
fn test_joint_with_joint_accepts_quorums_of_both_sides() {
    let mut joints = all_joints();
    // Joints over IDs 0 to 3 keep this check short: pairs of joints over all 5 IDs take about 60
    // seconds.
    joints.retain(|joint| joint.iter().all(|config| !config.contains(&4)));
    let id_sets = all_configs();

    for current in &joints {
        for goal in &joints {
            let joint = current.joint_with(goal.clone());
            for ids in &id_sets {
                let in_current = current.is_quorum(ids.iter());
                let in_goal = goal.is_quorum(ids.iter());
                let in_both = in_current && in_goal;
                let in_joint = joint.is_quorum(ids.iter());
                assert_eq!(
                    in_both, in_joint,
                    "{current:?} joint with {goal:?} on {ids:?}"
                );
            }
        }
    }
}
