use crate::quorum::QuorumSet;

/// Relation between quorum sets whose quorums always intersect.
///
/// **Coherent** quorum set A and B is defined as:
/// `∀ qᵢ ∈ A, ∀ qⱼ ∈ B: qᵢ ∩ qⱼ != ø`, i.e., `A ~ B`.
/// In words, every quorum in A intersects every quorum in B. Consensus
/// protocols use this relation to make membership changes without losing
/// overlap between old and new decisions.
///
/// In a Raft-style membership change, coherence is one safety requirement. The
/// protocol also has to prevent an old, smaller candidate from being elected
/// during the transition.
pub trait Coherent<ID, Other>
where
    ID: PartialOrd + Ord + 'static,
    Self: QuorumSet<Id = ID>,
    Other: QuorumSet<Id = ID>,
{
    /// Return `true` if this quorum set is coherent with the other quorum set.
    fn is_coherent_with(&self, other: &Other) -> bool;
}

/// Finds an intermediate quorum set coherent with both source and target quorum sets.
pub trait FindCoherent<ID, Other>
where
    ID: PartialOrd + Ord + 'static,
    Self: QuorumSet<Id = ID>,
    Other: QuorumSet<Id = ID>,
{
    /// Build a quorum set `X` so that `self` is coherent with `X` and `X` is
    /// coherent with `other`, i.e., `self ~ X ~ other`.
    ///
    /// Then `X` is the intermediate quorum set when changing membership from
    /// `self` to `other`.
    ///
    /// E.g.(`cᵢcⱼ` is a joint of `cᵢ` and `cⱼ`):
    /// - `c₁.find_coherent(c₁)`   returns `c₁`
    /// - `c₁.find_coherent(c₂)`   returns `c₁c₂`
    /// - `c₁c₂.find_coherent(c₂)` returns `c₂`
    /// - `c₁c₂.find_coherent(c₁)` returns `c₁`
    /// - `c₁c2.find_coherent(c₃)` returns `c₂c₃`
    fn find_coherent(&self, other: Other) -> Self;
}
