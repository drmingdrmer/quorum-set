use std::collections::BTreeSet;
use std::collections::btree_set;

use crate::quorum::GridQuorumSetError;
use crate::quorum::QuorumIntersection;
use crate::quorum::QuorumSet;

/// How a [`GridQuorumSet`] counts the rows of its grid: a full row, or one ID of each row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridRule {
    /// An accepted set holds every ID of some row.
    FullRow,

    /// An accepted set holds at least one ID of every row. Each row can contribute an ID from
    /// a different column.
    OnePerRow,
}

/// A quorum rule that accepts an ID set holding every ID of some row of its grid, or at least
/// one ID of every row.
///
/// The grid is a rectangle of distinct IDs, given as its rows. [`GridRule`] selects whether an
/// accepted set holds a full row or one ID of each row.
///
/// The rows of a grid are disjoint, so no row holds another, and the minimal quorums of the
/// `FullRow` rule are exactly the rows. The minimal quorums of the `OnePerRow` rule hold one ID
/// of each row, from any column. Each row meets every `OnePerRow` quorum of its grid, so the
/// `FullRow` rule and the `OnePerRow` rule of one grid always intersect.
///
/// # Examples
///
/// ```
/// use quorum_set::{GridQuorumSet, GridRule, QuorumIntersection, QuorumSet};
///
/// let grid = vec![vec![1, 2, 3], vec![4, 5, 6]];
/// let full_row = GridQuorumSet::new(grid.clone(), GridRule::FullRow).unwrap();
/// let one_per_row = GridQuorumSet::new(grid, GridRule::OnePerRow).unwrap();
///
/// let accepts = full_row.is_quorum([4, 5, 6].iter());
/// assert!(accepts);
/// // {1,5} and {3,4} each hold an ID of both rows, from different columns.
/// let accepts = one_per_row.is_quorum([1, 5].iter());
/// assert!(accepts);
/// let accepts = one_per_row.is_quorum([3, 4].iter());
/// assert!(accepts);
/// // {1,2,3} holds no ID of row {4,5,6}.
/// let accepts = one_per_row.is_quorum([1, 2, 3].iter());
/// assert!(!accepts);
///
/// // Every row meets every `OnePerRow` quorum, but row {1,2,3} misses row {4,5,6}.
/// let intersects = full_row.intersects_with(&one_per_row);
/// assert_eq!(Some(true), intersects);
/// let intersects = full_row.intersects_with(&full_row);
/// assert_eq!(Some(false), intersects);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridQuorumSet<ID> {
    /// The rows of the grid, each one the IDs of its cells.
    grid: Vec<Vec<ID>>,

    /// Whether an accepted set holds a full row or one ID of each row.
    rule: GridRule,
}

impl<ID> GridQuorumSet<ID>
where ID: Ord + Clone
{
    /// Builds a rule over `grid`, given as its rows, that accepts an ID set holding a full row or
    /// one ID of each row, as `rule` selects.
    ///
    /// Returns [`GridQuorumSetError`] unless `grid` has a row and a column, every row is as long
    /// as the first, and no ID is in two cells.
    pub fn new(grid: Vec<Vec<ID>>, rule: GridRule) -> Result<Self, GridQuorumSetError<ID>> {
        check_grid(&grid)?;
        let quorum_set = Self { grid, rule };
        Ok(quorum_set)
    }

    /// Returns the grid, as its rows.
    pub fn grid(&self) -> &[Vec<ID>] {
        &self.grid
    }

    /// Returns whether an accepted set holds a full row or one ID of each row.
    pub fn rule(&self) -> GridRule {
        self.rule
    }

    /// Returns whether `rule` over `grid` accepts `candidates`.
    fn accepts(grid: &[Vec<ID>], rule: GridRule, candidates: &BTreeSet<&ID>) -> bool {
        match rule {
            GridRule::FullRow => {
                for row in grid.iter() {
                    let holds_row = row.iter().all(|id| candidates.contains(id));
                    if holds_row {
                        return true;
                    }
                }
                false
            }
            GridRule::OnePerRow => {
                for row in grid.iter() {
                    let meets_row = row.iter().any(|id| candidates.contains(id));
                    if !meets_row {
                        return false;
                    }
                }
                true
            }
        }
    }
}

impl<ID> QuorumSet for GridQuorumSet<ID>
where ID: Ord + Clone
{
    type Id = ID;
    type Iter = btree_set::IntoIter<ID>;

    fn is_quorum<'a, I>(&self, ids: I) -> bool
    where
        ID: 'a,
        I: Iterator<Item = &'a ID> + Clone,
    {
        let candidates: BTreeSet<&ID> = ids.collect();
        Self::accepts(&self.grid, self.rule, &candidates)
    }

    fn ids(&self) -> Self::Iter {
        let cells = self.grid.iter().flatten();
        let ids: BTreeSet<ID> = cells.cloned().collect();
        ids.into_iter()
    }
}

/// Quorum intersection for two grid rules, exact when either rule is [`GridRule::FullRow`].
///
/// A set meets every quorum of a rule exactly when the other [`GridRule`] over the same grid
/// accepts the set:
///
/// - A set meets every row exactly when it holds an ID of every row.
/// - A set meets every `OnePerRow` quorum exactly when it holds a full row. A set without a full
///   row misses an ID of each row, and these IDs form a `OnePerRow` quorum that the set misses.
///
/// Every quorum of a `FullRow` rule holds one of its rows, and every row is a quorum. A `FullRow`
/// rule therefore intersects another rule exactly when each of its rows meets every quorum of the
/// other rule. The check gives no answer for two `OnePerRow` rules. A `FullRow` read rule with a
/// `OnePerRow` write rule over one grid, or the reverse, always gets an exact answer.
impl<ID> QuorumIntersection<GridQuorumSet<ID>> for GridQuorumSet<ID>
where ID: Ord + Clone
{
    /// Return `Some(false)` when a row of a `FullRow` rule misses a quorum of the other rule, and
    /// `Some(true)` otherwise. Return `None` for two `OnePerRow` rules.
    fn intersects_with(&self, other: &GridQuorumSet<ID>) -> Option<bool> {
        let (full_row, compared) = match (self.rule, other.rule) {
            (GridRule::FullRow, _) => (self, other),
            (_, GridRule::FullRow) => (other, self),
            (GridRule::OnePerRow, GridRule::OnePerRow) => return None,
        };

        // A set meets every quorum of `compared` exactly when the other rule over its grid accepts
        // the set.
        let meeting_rule = match compared.rule {
            GridRule::FullRow => GridRule::OnePerRow,
            GridRule::OnePerRow => GridRule::FullRow,
        };
        for row in full_row.grid.iter() {
            let candidates = row.iter().collect();
            let meets_every_quorum = Self::accepts(&compared.grid, meeting_rule, &candidates);
            if !meets_every_quorum {
                return Some(false);
            }
        }
        Some(true)
    }
}

/// Returns an error unless `grid` has a row and a column, every row is as long as the first, and
/// no ID is in two cells.
fn check_grid<ID>(grid: &[Vec<ID>]) -> Result<(), GridQuorumSetError<ID>>
where ID: Ord + Clone {
    let Some(first_row) = grid.first() else {
        return Err(GridQuorumSetError::NoRow);
    };
    let column_count = first_row.len();
    if column_count == 0 {
        return Err(GridQuorumSetError::NoColumn);
    }

    for (row, cells) in grid.iter().enumerate() {
        let len = cells.len();
        if len != column_count {
            let ragged = GridQuorumSetError::RaggedRow {
                row,
                len,
                column_count,
            };
            return Err(ragged);
        }
    }

    let mut seen = BTreeSet::new();
    for id in grid.iter().flatten() {
        let is_new = seen.insert(id);
        if !is_new {
            let duplicate = GridQuorumSetError::DuplicateId { id: id.clone() };
            return Err(duplicate);
        }
    }
    Ok(())
}
