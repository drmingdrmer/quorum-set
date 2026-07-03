use std::fmt;

use sha2::Digest;
use sha2::Sha256;

use crate::QuorumTree;
use crate::canonical_id::CanonicalId;
use crate::canonical_id::MAX_CANONICAL_ID_LEN;
use crate::canonical_id::fmt_escaped;

/// Either a node ID or a nested quorum set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuorumNode<ID>
where ID: Ord
{
    Id(ID),
    Set(QuorumTree<ID>),
}

impl<ID> QuorumNode<ID>
where ID: Ord
{
    /// whether the `ids` select this node or not.
    ///
    /// If it is a single node, it means this node ID is included in the
    /// provided IDs. If it is a nested quorum set, being selected means the
    /// IDs form a quorum of the nested quorum set.
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
