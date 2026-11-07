use std::error::Error;
use std::fmt;

#[cfg(doc)]
use crate::WeightedQuorumSet;

/// An error returned when building an invalid [`WeightedQuorumSet`].
///
/// Both cases are caller bugs: an ID with weight 0 would never count, and a threshold of 0 would
/// make the empty set a quorum. Construction reports them instead of repairing the input, so the
/// mistake surfaces where it is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightedQuorumSetError<ID> {
    /// An ID has weight 0, so it would never count.
    ZeroWeight {
        /// The first ID with weight 0.
        id: ID,
    },

    /// The threshold is 0, so the empty set would be a quorum.
    ZeroThreshold,
}

impl<ID> fmt::Display for WeightedQuorumSetError<ID>
where ID: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWeight { id } => {
                write!(f, "ID {id} has weight 0; every weight must be positive")
            }
            Self::ZeroThreshold => {
                write!(f, "the threshold is 0; it must be positive")
            }
        }
    }
}

impl<ID> Error for WeightedQuorumSetError<ID> where ID: fmt::Debug + fmt::Display {}

#[cfg(test)]
mod tests {
    use super::WeightedQuorumSetError;

    #[test]
    fn test_display() {
        let err = WeightedQuorumSetError::ZeroWeight { id: 2 };
        assert_eq!(
            "ID 2 has weight 0; every weight must be positive",
            err.to_string()
        );

        let err = WeightedQuorumSetError::<u64>::ZeroThreshold;
        assert_eq!("the threshold is 0; it must be positive", err.to_string());
    }
}
