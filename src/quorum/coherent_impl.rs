use std::collections::BTreeSet;

use crate::quorum::Coherent;
use crate::quorum::coherent::FindCoherent;

/// Two joint configs are treated as coherent when they share at least one config: every quorum of
/// one then intersects every quorum of the other. Sharing a config is sufficient but not necessary
/// for that property, so this check can return `false` for a pair of configs whose quorums do in
/// fact all intersect.
impl<NID> Coherent<NID, Vec<BTreeSet<NID>>> for Vec<BTreeSet<NID>>
where NID: PartialOrd + Ord + Clone + 'static
{
    /// Return `true` when two joint quorum sets share a config.
    ///
    /// Read more about extended membership change in OpenRaft:
    /// <https://docs.rs/openraft/latest/openraft/docs/data/extended_membership/index.html>
    fn is_coherent_with(&self, other: &Vec<BTreeSet<NID>>) -> bool {
        for a in self {
            for b in other {
                if a == b {
                    return true;
                }
            }
        }
        false
    }
}

/// Builds an intermediate joint quorum set for a target flat config.
impl<NID> FindCoherent<NID, BTreeSet<NID>> for Vec<BTreeSet<NID>>
where NID: PartialOrd + Ord + Clone + 'static
{
    fn find_coherent(&self, other: BTreeSet<NID>) -> Self {
        if self.is_coherent_with(&vec![other.clone()]) {
            vec![other]
        } else if let Some(last) = self.last() {
            vec![last.clone(), other]
        } else {
            vec![other]
        }
    }
}
