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

    /// Build the joint `X` of `self` and `other`: a set is a quorum of `X` exactly when it is a
    /// quorum of both `self` and `other`.
    ///
    /// A change between two memberships with separate read and write quorum sets builds the joint
    /// membership with two calls: one joins the old and the new read quorum sets, and one joins
    /// the old and the new write quorum sets. Every joint read quorum is then a read quorum of both
    /// the old and the new membership, and every joint write quorum is a write quorum of both.
    ///
    /// Let `~` be the [`QuorumIntersection`] relation. When `read ~ write` holds within the old
    /// and within the new membership, these hold too:
    /// - `old read ~ joint write` and `old write ~ joint read`
    /// - `joint read ~ new write` and `joint write ~ new read`
    /// - `joint read ~ joint write`
    ///
    /// E.g. `c₁c₂.joint_with(c₃)` returns `c₁c₂c₃`, where `cᵢcⱼ` is a joint of `cᵢ` and `cⱼ`.
    fn joint_with(&self, other: Other) -> Self;
}
