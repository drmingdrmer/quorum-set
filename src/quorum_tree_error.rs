use std::error::Error;
use std::fmt;

/// An error returned when building an invalid [`QuorumTree`](crate::QuorumTree).
///
/// Both cases are caller bugs: a correct quorum rule never contains the same
/// child twice and never requires more children than it has. Construction
/// reports them instead of repairing the input, so the mistake surfaces where
/// it is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumTreeError {
    /// The same child node was given more than once.
    DuplicateChild {
        /// Canonical ID of the duplicated child node.
        canonical_id: String,
    },

    /// `quorum_size` exceeds the number of child nodes, so no input could
    /// satisfy the tree.
    UnsatisfiableQuorum {
        /// The required number of selected children.
        quorum_size: u64,
        /// The number of child nodes.
        num_children: usize,
    },
}

impl fmt::Display for QuorumTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateChild { canonical_id } => {
                write!(f, "duplicate child node: {canonical_id}")
            }
            Self::UnsatisfiableQuorum {
                quorum_size,
                num_children,
            } => {
                write!(
                    f,
                    "quorum size {quorum_size} exceeds the number of child nodes {num_children}"
                )
            }
        }
    }
}

impl Error for QuorumTreeError {}

#[cfg(test)]
mod tests {
    use super::QuorumTreeError;

    #[test]
    fn test_display() {
        let err = QuorumTreeError::DuplicateChild {
            canonical_id: "Id=1".to_string(),
        };
        assert_eq!("duplicate child node: Id=1", err.to_string());

        let err = QuorumTreeError::UnsatisfiableQuorum {
            quorum_size: 3,
            num_children: 2,
        };
        assert_eq!(
            "quorum size 3 exceeds the number of child nodes 2",
            err.to_string()
        );
    }
}
