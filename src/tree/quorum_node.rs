use std::fmt;

use sha2::Digest;
use sha2::Sha256;

use super::canonical_id::CanonicalId;
use super::canonical_id::MAX_CANONICAL_ID_LEN;
use super::canonical_id::fmt_escaped;
use crate::QuorumTree;

/// A child of a [`QuorumTree`].
///
/// A node can be either a leaf node ID or a nested quorum tree. Nested trees
/// allow hierarchical quorum rules such as "two data centers, each selected by
/// a local majority".
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node<ID>
where ID: Ord
{
    /// A leaf node ID.
    Id(ID),

    /// A nested quorum tree.
    Subtree(QuorumTree<ID>),
}

impl<ID> Node<ID>
where ID: Ord
{
    /// Returns whether `ids` select this child node.
    ///
    /// A leaf node is selected when its ID is present in `ids`. A nested tree is
    /// selected when `ids` satisfy that tree.
    pub(crate) fn is_selected_by<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: IntoIterator<Item = &'a ID> + Clone,
    {
        match self {
            Node::Id(id) => ids.into_iter().any(|candidate| candidate == id),
            Node::Subtree(m) => m.spec.is_quorum(ids),
        }
    }
}

impl<ID> CanonicalId for Node<ID>
where
    ID: CanonicalId,
    ID: Ord,
{
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        match self {
            Node::Id(id) => {
                write!(f, "Id=")?;

                let id = id.canonical_id();
                if id.len() > MAX_CANONICAL_ID_LEN {
                    write!(f, "Hash#1:{:x}", Sha256::digest(id.as_bytes()))?;
                } else {
                    fmt_escaped(&id, f)?;
                }
            }
            Node::Subtree(m) => {
                write!(f, "Subtree=")?;
                m.fmt_canonical_id(f)?;
            }
        }
        Ok(())
    }
}
