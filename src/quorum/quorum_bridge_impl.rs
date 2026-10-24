use std::collections::BTreeSet;

use crate::quorum::QuorumBridge;
use crate::quorum::QuorumIntersection;

/// Builds an intermediate joint quorum set for a target flat config.
///
/// The result is the target alone when [`QuorumIntersection::intersects_with`] proves that every
/// quorum of `self` intersects every quorum of the target, e.g. when the target adds one voter to
/// a uniform `self`. Otherwise it is the joint of the last config of `self` and the target.
///
/// A Raft-style caller that applies a target-alone result must first commit an entry of the
/// leader's own term. Otherwise two leaders of different terms can each move from the same
/// committed config to a different target, and the two targets can have disjoint quorums.
impl<ID> QuorumBridge<BTreeSet<ID>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    fn bridge_to(&self, other: BTreeSet<ID>) -> Self {
        let intersects = self.intersects_with(&other);
        if intersects == Some(true) {
            vec![other]
        } else if let Some(last) = self.last() {
            vec![last.clone(), other]
        } else {
            vec![other]
        }
    }

    /// Return the configs of `self` followed by `other`. Return an empty joint when `self` is
    /// empty: an empty joint has no quorum, so a joint with it has no quorum either.
    fn joint_with(&self, other: BTreeSet<ID>) -> Self {
        if self.is_empty() {
            return vec![];
        }
        let mut joint = self.clone();
        joint.push(other);
        joint
    }
}

/// Builds an intermediate joint quorum set for a target joint config.
///
/// The result is the target alone when [`QuorumIntersection::intersects_with`] proves that every
/// quorum of `self` intersects every quorum of the target. Otherwise it is the joint of the last
/// config of `self` and the configs of the target. For a target that holds one config, the result
/// equals the result for that config as a [`BTreeSet`] target.
///
/// A Raft-style caller that applies a target-alone result must first commit an entry of the
/// leader's own term. Otherwise two leaders of different terms can each move from the same
/// committed config to a different target, and the two targets can have disjoint quorums.
impl<ID> QuorumBridge<Vec<BTreeSet<ID>>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    fn bridge_to(&self, other: Vec<BTreeSet<ID>>) -> Self {
        let intersects = self.intersects_with(&other);
        if intersects == Some(true) {
            other
        } else if let Some(last) = self.last() {
            let mut joint = vec![last.clone()];
            joint.extend(other);
            joint
        } else {
            other
        }
    }

    /// Return the configs of `self` followed by the configs of `other`. Return an empty joint
    /// when `self` or `other` is empty: an empty joint has no quorum, so a joint with it has no
    /// quorum either.
    fn joint_with(&self, other: Vec<BTreeSet<ID>>) -> Self {
        if self.is_empty() || other.is_empty() {
            return vec![];
        }
        let mut joint = self.clone();
        joint.extend(other);
        joint
    }
}
