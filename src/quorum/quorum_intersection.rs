use std::collections::BTreeSet;

#[cfg(doc)]
use crate::QuorumTree;
use crate::quorum::QuorumSet;

/// Relation between quorum sets whose quorums always intersect.
///
/// Quorum sets A and B have **quorum intersection**, written `A ~ B`, when:
/// `∀ qᵢ ∈ A, ∀ qⱼ ∈ B: qᵢ ∩ qⱼ != ø`.
/// In words, every quorum in A intersects every quorum in B. Consensus
/// protocols use this relation to make membership changes without losing
/// overlap between old and new decisions.
///
/// The relation is symmetric, and both universal quantifiers are load-bearing:
/// weakening either one to "some quorum" (∃) breaks safety, because a reader
/// or a candidate cannot know which quorum is "the right one" — the overlap
/// must hold for every quorum it may legally assemble. E.g. for write quorums
/// `{a,b}, {b,c}, {a,c}` and read quorums `{b,c}, {x,y}`: every write quorum
/// intersects *some* read quorum, yet a read using `{x,y}` observes no
/// committed write.
///
/// In a Raft-style membership change, quorum intersection is one safety
/// requirement. The protocol also has to prevent an old, smaller candidate
/// from being elected during the transition.
pub trait QuorumIntersection<Other>
where
    Self: QuorumSet,
    Other: QuorumSet<Id = Self::Id>,
{
    /// Return whether every quorum of this quorum set intersects every quorum
    /// of the other quorum set.
    ///
    /// - `Some(true)`: the check proved that every quorum pair intersects.
    /// - `Some(false)`: the check proved that some quorum pair is disjoint.
    /// - `None`: the check proved neither. An implementation may use a condition that is sufficient
    ///   but not necessary, so failing that condition tells the caller nothing about the true
    ///   relation.
    ///
    /// Callers can act on `Some(true)`. On `Some(false)` and on `None` they
    /// must take the unconditionally safe path, e.g. bridge through a joint
    /// config built by [`QuorumBridge`]. [`verify_intersection`] computes the
    /// exact relation in exponential time.
    fn intersects_with(&self, other: &Other) -> Option<bool>;
}

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
    /// E.g.(`cᵢcⱼ` is a joint of `cᵢ` and `cⱼ`):
    /// - `c₁.bridge_to(c₁)`   returns `c₁`
    /// - `c₁.bridge_to(c₂)`   returns `c₁c₂`
    /// - `c₁c₂.bridge_to(c₂)` returns `c₂`
    /// - `c₁c₂.bridge_to(c₁)` returns `c₁`
    /// - `c₁c₂.bridge_to(c₃)` returns `c₂c₃`
    fn bridge_to(&self, other: Other) -> Self;
}

/// Exhaustively check the [`QuorumIntersection`] relation between two quorum
/// sets.
///
/// Returns `true` iff every quorum of `a` intersects every quorum of `b`.
/// Unlike [`QuorumIntersection::intersects_with`], which may answer `None`,
/// this check is exact for any two [`QuorumSet`] implementations, e.g. a read
/// [`QuorumTree`] against a write [`QuorumTree`].
///
/// It tests every split of the combined voter IDs `U` into `(S, U ∖ S)`: a
/// disjoint quorum pair exists iff for some split, `S` is a quorum of `a` and
/// `U ∖ S` is a quorum of `b`. Quorum sets are upward-closed, so "some quorum
/// of `b` fits inside `U ∖ S`" is the same as "`U ∖ S` is itself a quorum of
/// `b`". This argument relies on IDs outside [`QuorumSet::ids`] never
/// affecting [`QuorumSet::is_quorum`], which holds for every implementation
/// in this crate.
///
/// The check runs `2^n` quorum evaluations for `n` distinct IDs. It is meant
/// for validating small configurations and as a test oracle.
///
/// # Panics
///
/// Panics if `a` and `b` together track 64 or more distinct IDs.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeSet;
///
/// use quorum_set::verify_intersection;
///
/// let abc = BTreeSet::from([1, 2, 3]);
/// let de = BTreeSet::from([4, 5]);
///
/// // Majorities of one voter set always intersect each other.
/// assert!(verify_intersection(&abc, &abc));
/// // Majorities of disjoint voter sets never intersect.
/// assert!(!verify_intersection(&abc, &de));
/// ```
pub fn verify_intersection<A, B>(a: &A, b: &B) -> bool
where
    A: QuorumSet,
    B: QuorumSet<Id = A::Id>,
    A::Id: Ord,
{
    let universe: BTreeSet<A::Id> = a.ids().chain(b.ids()).collect();
    let universe: Vec<A::Id> = universe.into_iter().collect();
    let n = universe.len();
    assert!(
        n < 64,
        "verify_intersection enumerates 2^n subsets; {n} distinct ids do not fit a u64 mask"
    );

    for mask in 0u64..(1u64 << n) {
        let selected = universe
            .iter()
            .enumerate()
            .filter(move |&(i, _)| mask & (1u64 << i) != 0)
            .map(|(_, id)| id);
        let complement = universe
            .iter()
            .enumerate()
            .filter(move |&(i, _)| mask & (1u64 << i) == 0)
            .map(|(_, id)| id);
        if a.is_quorum(selected) && b.is_quorum(complement) {
            return false;
        }
    }
    true
}
