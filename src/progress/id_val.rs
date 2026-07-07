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

#[cfg(test)]
mod tests {
    use super::IdVal;
    use crate::progress::VecProgressEntry;

    #[test]
    fn test_new_and_new_default() {
        assert_eq!(
            IdVal {
                id: 3u64,
                val: 7u64
            },
            IdVal::new(3, 7)
        );
        assert_eq!(IdVal::new(2u64, 0u64), IdVal::new_default(2));
    }

    #[test]
    fn test_display() {
        assert_eq!("3: 7", IdVal::new(3u64, 7u64).to_string());
    }

    #[test]
    fn test_vec_progress_entry_accessors() {
        let mut id_val = IdVal::new(3u64, 7u64);

        assert_eq!(&3, id_val.id());
        assert_eq!(&7, id_val.progress());

        *id_val.progress_mut() = 9;
        assert_eq!(IdVal::new(3, 9), id_val);
    }
}
