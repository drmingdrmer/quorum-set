use std::fmt;

use crate::progress::VecProgressEntry;

/// An ID and its associated value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdVal<ID, Val> {
    /// Node ID.
    pub id: ID,

    /// Associated progress value.
    pub val: Val,
}

impl<ID, Val> fmt::Display for IdVal<ID, Val>
where
    ID: fmt::Display,
    Val: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.val)
    }
}

impl<ID, Val> IdVal<ID, Val> {
    /// Create an [`IdVal`] with the provided ID and value.
    pub fn new(id: ID, val: Val) -> Self {
        Self { id, val }
    }
}

impl<ID, Val> IdVal<ID, Val>
where Val: Default
{
    /// Create an [`IdVal`] with the provided ID and `Val::default()`.
    pub fn new_default(id: ID) -> Self {
        Self::new(id, Default::default())
    }
}

impl<ID, Val> VecProgressEntry for IdVal<ID, Val>
where
    ID: 'static + PartialEq,
    Val: Clone + Default + Ord,
{
    type Id = ID;
    type Progress = Val;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn progress(&self) -> &Self::Progress {
        &self.val
    }

    fn progress_mut(&mut self) -> &mut Self::Progress {
        &mut self.val
    }
}
