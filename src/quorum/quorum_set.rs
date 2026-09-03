use std::sync::Arc;

#[cfg(doc)]
use crate::QuorumTree;
#[cfg(doc)]
use crate::VecProgress;
#[cfg(doc)]
use crate::verify_intersection;

/// Common interface for every quorum rule supported by this crate.
///
/// A quorum is a collection of nodes that a read or write operation in a distributed system has to
/// contact. See: <http://web.mit.edu/6.033/2005/wwwdocs/quorum_note.html>
///
/// Every implementation must follow three rules:
///
/// - Upward-closed: adding IDs to an accepted quorum keeps it accepted. [`VecProgress`] and
///   [`verify_intersection`] rely on this rule.
/// - Closed over [`ids()`](Self::ids): an ID that `ids()` does not yield never changes the result
///   of [`is_quorum()`](Self::is_quorum). [`verify_intersection`] enumerates only the IDs that
///   `ids()` yields, so it relies on this rule.
/// - Duplicate-safe: an ID that appears more than once in the `is_quorum()` input counts once, so
///   callers may pass an iterator with repeats.
///
/// The crate provides implementations for flat majority sets, joint quorum sets, and hierarchical
/// [`QuorumTree`] rules.
pub trait QuorumSet {
    /// Node ID type in this quorum set.
    type Id;

    /// Iterator over every voter ID tracked by this quorum set.
    ///
    /// Implementations that combine multiple sub-rules return each ID once.
    type Iter: Iterator<Item = Self::Id>;

    /// Return `true` if the candidate IDs satisfy this quorum rule.
    ///
    /// Repeated IDs count once, and IDs that [`ids()`](Self::ids) does not yield are ignored.
    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        Self::Id: 'a,
        I: Iterator<Item = &'a Self::Id> + Clone;

    /// Return all voter IDs in this quorum set.
    fn ids(&self) -> Self::Iter;
}

impl<T> QuorumSet for Arc<T>
where T: QuorumSet
{
    type Id = T::Id;

    type Iter = T::Iter;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        Self::Id: 'a,
        I: Iterator<Item = &'a Self::Id> + Clone,
    {
        self.as_ref().is_quorum(ids)
    }

    fn ids(&self) -> Self::Iter {
        self.as_ref().ids()
    }
}
