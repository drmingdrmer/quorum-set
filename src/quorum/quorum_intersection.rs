use crate::quorum::QuorumSet;

/// Relation between quorum sets whose quorums always intersect.
///
/// Quorum sets A and B have **quorum intersection**, written `A ~ B`, when:
/// `∀ qᵢ ∈ A, ∀ qⱼ ∈ B: qᵢ ∩ qⱼ != ø`.
/// In words, every quorum in A intersects every quorum in B. Consensus
/// protocols use this relation to make membership changes without losing
/// overlap between old and new decisions.
///
/// In a Raft-style membership change, quorum intersection is one safety
/// requirement. The protocol also has to prevent an old, smaller candidate
/// from being elected during the transition.
pub trait QuorumIntersection<ID, Other>
where
    ID: PartialOrd + Ord + 'static,
    Self: QuorumSet<Id = ID>,
    Other: QuorumSet<Id = ID>,
{
    /// Return `true` if every quorum of this quorum set intersects every
    /// quorum of the other quorum set.
    fn intersects_with(&self, other: &Other) -> bool;
}

/// Builds an intermediate quorum set that has [`QuorumIntersection`] with both
/// the source and the target quorum set.
pub trait QuorumBridge<ID, Other>
where
    ID: PartialOrd + Ord + 'static,
    Self: QuorumSet<Id = ID>,
    Other: QuorumSet<Id = ID>,
{
    /// Build a quorum set `X` so that `self ~ X ~ other`, where `~` is the
    /// [`QuorumIntersection`] relation.
    ///
    /// Then `X` is the intermediate quorum set when changing membership from
    /// `self` to `other`.
    ///
    /// E.g.(`cᵢcⱼ` is a joint of `cᵢ` and `cⱼ`):
    /// - `c₁.bridge_to(c₁)`   returns `c₁`
    /// - `c₁.bridge_to(c₂)`   returns `c₁c₂`
    /// - `c₁c₂.bridge_to(c₂)` returns `c₂`
    /// - `c₁c₂.bridge_to(c₁)` returns `c₁`
    /// - `c₁c₂.bridge_to(c₃)` returns `c₂c₃`
    fn bridge_to(&self, other: Other) -> Self;
}
