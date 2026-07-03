use std::fmt;

use sha2::Digest;
use sha2::Sha256;

use crate::QuorumTree;
use crate::canonical_id::CanonicalId;
use crate::canonical_id::MAX_CANONICAL_ID_LEN;
use crate::canonical_id::fmt_escaped;

/// A child of a [`QuorumTree`](crate::QuorumTree).
///
/// A node can be either a leaf node ID or a nested quorum tree. Nested trees
/// allow hierarchical quorum rules such as "two data centers, each selected by
/// a local majority".
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuorumNode<ID>
where ID: Ord
{
    /// A leaf node ID.
    Id(ID),

    /// A nested quorum tree.
    Set(QuorumTree<ID>),
}

impl<ID> QuorumNode<ID>
where ID: Ord
{
    /// Returns whether `ids` select this child node.
    ///
    /// A leaf node is selected when its ID is present in `ids`. A nested tree is
    /// selected when `ids` satisfy that tree.
    pub fn is_selected(&self, ids: &[ID]) -> bool {
        match self {
            QuorumNode::Id(id) => ids.contains(id),
            QuorumNode::Set(m) => m.is_quorum(ids),
        }
    }
}

impl<ID> CanonicalId for QuorumNode<ID>
where
    ID: CanonicalId,
    ID: Ord,
{
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        match self {
            QuorumNode::Id(id) => {
                write!(f, "Id=")?;

                let id = id.canonical_id();
                if id.len() > MAX_CANONICAL_ID_LEN {
                    write!(f, "Hash#1:{:x}", Sha256::digest(id.as_bytes()))?;
                } else {
                    fmt_escaped(&id, f)?;
                }
            }
            QuorumNode::Set(m) => {
                write!(f, "Set=")?;
                m.fmt_canonical_id(f)?;
            }
        }
        Ok(())
    }
}
