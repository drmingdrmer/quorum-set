use std::error::Error;
use std::fmt;

#[cfg(doc)]
use crate::GridQuorumSet;

/// An error returned when building an invalid [`GridQuorumSet`].
///
/// A grid must be a rectangle of distinct IDs with at least one row and one column. Construction
/// reports the first violation instead of repairing the input, so the mistake surfaces where it is
/// made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridQuorumSetError<ID> {
    /// The grid has no row, so it has no ID.
    NoRow,

    /// Row 0 is empty, so the grid has no column.
    NoColumn,

    /// A row is shorter or longer than row 0, so the grid is not a rectangle.
    RaggedRow {
        /// The index of the first row whose length differs from the length of row 0.
        row: usize,

        /// The length of that row.
        len: usize,

        /// The length of row 0, the number of columns.
        column_count: usize,
    },

    /// An ID is in more than one cell. Each cell holds its own ID.
    DuplicateId {
        /// The ID of the first cell, in row-major order, that repeats an earlier cell.
        id: ID,
    },
}

impl<ID> fmt::Display for GridQuorumSetError<ID>
where ID: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRow => write!(f, "the grid has no row"),
            Self::NoColumn => write!(f, "the grid has no column"),
            Self::RaggedRow {
                row,
                len,
                column_count,
            } => {
                write!(
                    f,
                    "row {row} has length {len}, but row 0 has length {column_count}"
                )
            }
            Self::DuplicateId { id } => write!(f, "ID {id} is in more than one cell"),
        }
    }
}

impl<ID> Error for GridQuorumSetError<ID> where ID: fmt::Debug + fmt::Display {}

#[cfg(test)]
mod tests {
    use super::GridQuorumSetError;

    #[test]
    fn test_display() {
        let err = GridQuorumSetError::<u64>::NoRow;
        assert_eq!("the grid has no row", err.to_string());

        let err = GridQuorumSetError::<u64>::NoColumn;
        assert_eq!("the grid has no column", err.to_string());

        let err = GridQuorumSetError::<u64>::RaggedRow {
            row: 1,
            len: 1,
            column_count: 2,
        };
        assert_eq!(
            "row 1 has length 1, but row 0 has length 2",
            err.to_string()
        );

        let err = GridQuorumSetError::DuplicateId { id: 3 };
        assert_eq!("ID 3 is in more than one cell", err.to_string());
    }
}
