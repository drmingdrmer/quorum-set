use std::error::Error;
use std::fmt;

#[cfg(doc)]
use crate::HierarchicalQuorumSet;

/// An error returned when building an invalid [`HierarchicalQuorumSet`].
///
/// Each variant names a group by its path: the names of the groups from the root down to it. The
/// path of the root group is empty. Construction reports the first problem instead of repairing
/// the input, so the mistake surfaces where it is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchicalQuorumSetError<ID> {
    /// A group holds no ID and no child group, so no set reaches it.
    EmptyGroup {
        /// The path of the empty group.
        group: Vec<String>,
    },

    /// Two groups hold the same ID, but each ID belongs to exactly one group of the tree.
    SharedId {
        /// The ID that both groups hold.
        id: ID,

        /// The path of the group that the check visits first. It visits a group before the groups
        /// below it, and sibling groups in name order.
        first: Vec<String>,

        /// The path of the group that the check visits second.
        second: Vec<String>,
    },

    /// The number of members that a quorum must reach in a group is outside `1..=member_count`.
    InvalidQuorumSize {
        /// The path of the group.
        group: Vec<String>,

        /// The number of members of the group that a quorum must reach.
        quorum_size: u64,

        /// The number of members of the group: its IDs and its child groups.
        member_count: u64,
    },
}

impl<ID> fmt::Display for HierarchicalQuorumSetError<ID>
where ID: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGroup { group } => write!(f, "group {group:?} holds no member"),
            Self::SharedId { id, first, second } => {
                write!(f, "groups {first:?} and {second:?} both hold ID {id}")
            }
            Self::InvalidQuorumSize {
                group,
                quorum_size,
                member_count,
            } => write!(
                f,
                "a quorum must reach {quorum_size} of the {member_count} members of group \
                 {group:?}; need 1 <= quorum_size <= member_count"
            ),
        }
    }
}

impl<ID> Error for HierarchicalQuorumSetError<ID> where ID: fmt::Debug + fmt::Display {}

#[cfg(test)]
mod tests {
    use super::HierarchicalQuorumSetError;

    #[test]
    fn test_display() {
        let err = HierarchicalQuorumSetError::<u64>::EmptyGroup {
            group: vec!["a".to_string()],
        };
        assert_eq!(r#"group ["a"] holds no member"#, err.to_string());

        let err = HierarchicalQuorumSetError::SharedId {
            id: 2,
            first: vec!["a".to_string()],
            second: vec![],
        };
        assert_eq!(r#"groups ["a"] and [] both hold ID 2"#, err.to_string());

        let err = HierarchicalQuorumSetError::<u64>::InvalidQuorumSize {
            group: vec![],
            quorum_size: 3,
            member_count: 2,
        };
        let want = "a quorum must reach 3 of the 2 members of group []; \
                    need 1 <= quorum_size <= member_count";
        assert_eq!(want, err.to_string());
    }
}
