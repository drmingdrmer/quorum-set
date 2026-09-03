use std::collections::BTreeSet;

use crate::quorum::QuorumBridge;
use crate::quorum::QuorumIntersection;

/// Two joint configs are treated as having quorum intersection when they share at least one
/// config: every quorum of one then intersects every quorum of the other. Sharing a config is
/// sufficient but not necessary for that property, so this check answers `None` for a pair of
/// configs whose quorums do in fact all intersect.
impl<ID> QuorumIntersection<Vec<BTreeSet<ID>>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    /// Return `Some(true)` when two joint quorum sets share a config.
    ///
    /// Return `Some(false)` when either joint is empty: an empty joint accepts the empty set as a
    /// quorum, and the empty set intersects nothing.
    ///
    /// Return `None` for every other pair, because the absence of a shared config proves nothing.
    ///
    /// Read more about extended membership change in OpenRaft:
    /// <https://docs.rs/openraft/latest/openraft/docs/data/extended_membership/index.html>
    fn intersects_with(&self, other: &Vec<BTreeSet<ID>>) -> Option<bool> {
        let either_is_empty = self.is_empty() || other.is_empty();
        if either_is_empty {
            return Some(false);
        }

        for a in self {
            for b in other {
                if a == b {
                    return Some(true);
                }
            }
        }
        None
    }
}

/// Builds an intermediate joint quorum set for a target flat config.
impl<ID> QuorumBridge<BTreeSet<ID>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    fn bridge_to(&self, other: BTreeSet<ID>) -> Self {
        let intersects = self.intersects_with(&vec![other.clone()]);
        if intersects == Some(true) {
            vec![other]
        } else if let Some(last) = self.last() {
            vec![last.clone(), other]
        } else {
            vec![other]
        }
    }
}
