use std::collections::BTreeSet;

use crate::quorum::QuorumBridge;

/// Builds the joint of a joint quorum set and a flat config.
impl<ID> QuorumBridge<BTreeSet<ID>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
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

/// Builds the joint of the source and target joint configs.
impl<ID> QuorumBridge<Vec<BTreeSet<ID>>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
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
