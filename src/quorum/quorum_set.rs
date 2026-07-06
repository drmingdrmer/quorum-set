use std::sync::Arc;

#[cfg(doc)]
use crate::QuorumTree;

/// Common interface for every quorum rule supported by this crate.
///
/// A quorum is a collection of nodes that a read or write operation in a distributed system has to
/// contact. See: <http://web.mit.edu/6.033/2005/wwwdocs/quorum_note.html>
///
/// Implementations must be upward-closed: adding IDs to an accepted quorum must keep it accepted.
/// The crate provides implementations for flat majority sets, joint quorum sets, and hierarchical
/// [`QuorumTree`] rules.
pub trait QuorumSet {
    /// Node ID type in this quorum set.
    type Id: 'static;

    /// Iterator over every voter ID tracked by this quorum set.
    ///
    /// Implementations that combine multiple sub-rules return each ID once.
    type Iter: Iterator<Item = Self::Id>;

    /// Return `true` if the candidate IDs satisfy this quorum rule.
    fn is_quorum<'a, I: Iterator<Item = &'a Self::Id> + Clone>(&self, ids: I) -> bool;

    /// Return all voter IDs in this quorum set.
    fn ids(&self) -> Self::Iter;
}

impl<T: QuorumSet> QuorumSet for Arc<T> {
    type Id = T::Id;

    type Iter = T::Iter;

    fn is_quorum<'a, I: Iterator<Item = &'a Self::Id> + Clone>(&self, ids: I) -> bool {
        self.as_ref().is_quorum(ids)
    }

    fn ids(&self) -> Self::Iter {
        self.as_ref().ids()
    }
}
