use std::collections::BTreeSet;

use crate::quorum::QuorumIntersection;

/// Two joint configs are treated as having quorum intersection when some config of one intersects
/// some config of the other: every quorum of a joint is also a quorum of each of its configs, so
/// every quorum of one joint then intersects every quorum of the other. This is sufficient but not
/// necessary for that property, so this check answers `None` for some pairs of joints whose
/// quorums do in fact all intersect.
impl<ID> QuorumIntersection<Vec<BTreeSet<ID>>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    /// Return the exact answer of the flat [`BTreeSet`] check when each joint holds one config.
    /// For other joints, return `Some(true)` when some config of `self` intersects some config of
    /// `other`.
    ///
    /// When either joint is empty, return `Some(true)` exactly when the other joint holds an empty
    /// config. An empty joint accepts the empty set as a quorum, and the empty set misses every
    /// quorum of the other joint. A joint that holds an empty config has no quorum to miss.
    ///
    /// Return `None` for every other pair: two configs that do not intersect prove nothing about
    /// two joints that contain them.
    ///
    /// Read more about extended membership change in OpenRaft:
    /// <https://docs.rs/openraft/latest/openraft/docs/data/extended_membership/index.html>
    fn intersects_with(&self, other: &Vec<BTreeSet<ID>>) -> Option<bool> {
        let either_is_empty = self.is_empty() || other.is_empty();
        if either_is_empty {
            let empty_config = BTreeSet::new();
            let holds_empty_config = self.contains(&empty_config) || other.contains(&empty_config);
            return Some(holds_empty_config);
        }

        let both_uniform = self.len() == 1 && other.len() == 1;
        if both_uniform {
            return self[0].intersects_with(&other[0]);
        }

        for a in self {
            for b in other {
                let intersects = a.intersects_with(b);
                if intersects == Some(true) {
                    return Some(true);
                }
            }
        }
        None
    }
}

/// Exact quorum intersection for two flat majority configs.
///
/// A smallest majority of `self` first takes the IDs that `other` lacks, and then takes shared IDs
/// only as needed. A smallest majority of `other` does the same. Two disjoint majorities exist
/// exactly when the shared IDs are enough for both, so the check is exact and runs in O(n).
impl<ID> QuorumIntersection<BTreeSet<ID>> for BTreeSet<ID>
where ID: Ord + Clone
{
    /// Return `Some(true)` when every majority of `self` intersects every majority of `other`,
    /// and `Some(false)` otherwise. This check never returns `None`.
    ///
    /// An empty [`BTreeSet`] has no quorum, so it has quorum intersection with every config.
    fn intersects_with(&self, other: &BTreeSet<ID>) -> Option<bool> {
        let majority_self = self.len() / 2 + 1;
        let majority_other = other.len() / 2 + 1;

        let only_self = self.difference(other).count();
        let only_other = other.difference(self).count();
        let shared = self.intersection(other).count();

        let need_self = majority_self.saturating_sub(only_self);
        let need_other = majority_other.saturating_sub(only_other);

        Some(need_self + need_other > shared)
    }
}

/// A joint config and a flat config are treated as having quorum intersection when some config of
/// the joint intersects the flat config: every quorum of the joint is also a quorum of that config.
/// This is sufficient but not necessary for that property, so this check answers `None` for some
/// pairs whose quorums do in fact all intersect.
impl<ID> QuorumIntersection<BTreeSet<ID>> for Vec<BTreeSet<ID>>
where ID: Ord + Clone
{
    /// Return the exact answer of the flat [`BTreeSet`] check when `self` holds one config. For
    /// other joints, return `Some(true)` when some config of `self` intersects `other`.
    ///
    /// Return `Some(other.is_empty())` when `self` is empty: an empty joint accepts the empty set
    /// as a quorum, and the empty set misses every quorum of a non-empty `other`. An empty `other`
    /// has no quorum to miss.
    ///
    /// Return `None` for every other pair.
    fn intersects_with(&self, other: &BTreeSet<ID>) -> Option<bool> {
        if self.is_empty() {
            return Some(other.is_empty());
        }

        let uniform = self.len() == 1;
        if uniform {
            return self[0].intersects_with(other);
        }

        for config in self {
            let intersects = config.intersects_with(other);
            if intersects == Some(true) {
                return Some(true);
            }
        }
        None
    }
}

/// Quorum intersection is symmetric, so this check returns the answer of the joint config's check.
impl<ID> QuorumIntersection<Vec<BTreeSet<ID>>> for BTreeSet<ID>
where ID: Ord + Clone
{
    fn intersects_with(&self, other: &Vec<BTreeSet<ID>>) -> Option<bool> {
        other.intersects_with(self)
    }
}
