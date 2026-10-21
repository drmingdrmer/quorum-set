#[cfg(doc)]
use crate::QuorumIntersection;
use crate::quorum::QuorumSet;

/// Builds an intermediate quorum set that has [`QuorumIntersection`] with both
/// the source and the target quorum set.
pub trait QuorumBridge<Other>
where
    Self: QuorumSet,
    Other: QuorumSet<Id = Self::Id>,
{
    /// Build a quorum set `X` so that `self ~ X ~ other`, where `~` is the
    /// [`QuorumIntersection`] relation.
    ///
    /// Then `X` is the intermediate quorum set when changing membership from
    /// `self` to `other`.
    ///
    /// E.g.(`cᵢcⱼ` is a joint of `cᵢ` and `cⱼ`, and no two distinct configs among `c₁`, `c₂`,
    /// `c₃` have intersecting quorums):
    /// - `c₁.bridge_to(c₁)`   returns `c₁`
    /// - `c₁.bridge_to(c₂)`   returns `c₁c₂`
    /// - `c₁c₂.bridge_to(c₂)` returns `c₂`
    /// - `c₁c₂.bridge_to(c₁)` returns `c₁`
    /// - `c₁c₂.bridge_to(c₃)` returns `c₂c₃`
    fn bridge_to(&self, other: Other) -> Self;
}
