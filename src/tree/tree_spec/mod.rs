use std::collections::BTreeSet;

use super::canonical_id::CanonicalId;
use crate::Node;
use crate::QuorumTreeError;

mod impl_canonical_id;
mod impl_display;

#[derive(Clone, Debug)]
pub(crate) struct QuorumTreeSpec<ID>
where ID: Ord
{
    quorum_size: u64,

    nodes: BTreeSet<Node<ID>>,
}

impl<ID> QuorumTreeSpec<ID>
where ID: Ord
{
    pub(crate) fn new(
        quorum_size: u64,
        nodes: impl IntoIterator<Item = Node<ID>>,
    ) -> Result<Self, QuorumTreeError>
    where
        ID: CanonicalId,
    {
        let mut unique = BTreeSet::new();
        for node in nodes {
            if let Some(dup) = unique.replace(node) {
                return Err(QuorumTreeError::DuplicateChild {
                    canonical_id: dup.canonical_id(),
                });
            }
        }

        if quorum_size > unique.len() as u64 {
            return Err(QuorumTreeError::UnsatisfiableQuorum {
                quorum_size,
                num_children: unique.len(),
            });
        }

        Ok(Self {
            quorum_size,
            nodes: unique,
        })
    }

    pub(crate) fn quorum_size(&self) -> u64 {
        self.quorum_size
    }

    pub(crate) fn children(&self) -> impl Iterator<Item = &Node<ID>> {
        self.nodes.iter()
    }

    pub(crate) fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: IntoIterator<Item = &'a ID> + Clone,
    {
        let required = self.quorum_size();
        if required == 0 {
            return true;
        }

        let mut count = 0;
        for node in &self.nodes {
            if node.is_selected_by(ids.clone()) {
                count += 1;
            }
            if count >= required {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tree_spec_test;
