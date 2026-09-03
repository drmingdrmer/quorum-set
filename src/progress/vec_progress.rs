use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::slice::Iter;
use std::slice::IterMut;

use validit::Validate;

use super::VecProgressEntry;
use super::VecProgressEntryData;
use super::display_vec_progress::DisplayVecProgress;
use super::progress_stats::ProgressStats;
use crate::quorum::QuorumSet;

/// Tracks per-node progress and the greatest value accepted by a quorum.
///
/// `Entry` stores a node ID, an ordered progress value, and optional
/// application-owned data. `QS` decides which node IDs constitute a quorum. In
/// Raft terms, this is a compact map from node ID to replicated log ID plus any
/// follower state the application keeps beside it.
///
/// Internally this type uses a vector and keeps only the voter prefix above the
/// current quorum-accepted value sorted. Normal updates may only keep or
/// increase progress; explicit resets may move an entry backward without
/// lowering the recorded quorum-accepted value. This makes the type a good fit
/// for small consensus memberships.
#[derive(Clone, Debug)]
pub struct VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Quorum set used to decide whether candidate IDs constitute a quorum.
    quorum_set: QS,

    /// The greatest value accepted by a quorum.
    quorum_accepted: Entry::Progress,

    /// Number of voter entries.
    voter_count: usize,

    /// Progress data.
    ///
    /// Elements with values greater than `quorum_accepted` are sorted in descending order.
    /// Others are unsorted.
    ///
    /// The first `voter_count` entries are voters; the rest are learners.
    /// Learners are not reordered by progress updates.
    /// Voters may move within the voter range to maintain the sorted prefix.
    entries: Vec<Entry>,

    /// Statistics of how it runs.
    stat: ProgressStats,
}

impl<Entry, QS> Display for VecProgress<Entry, QS>
where
    Entry: VecProgressEntry + Display,
    QS: QuorumSet<Id = Entry::Id>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        for (i, item) in self.entries.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", item)?
        }
        write!(f, "}}")?;

        Ok(())
    }
}

impl<Entry, QS> VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    Entry::Id: Ord + Clone + Debug,
    Entry::Progress: Debug,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Create a progress tracker from a quorum set and learner IDs.
    ///
    /// Voters are created from `quorum_set.ids()`. Learners are tracked after
    /// voters and never contribute to quorum acceptance. Every ID is tracked
    /// once: a learner ID that `quorum_set.ids()` also yields is a voter, and
    /// repeated IDs are ignored. `default_entry` builds the initial entry for
    /// every voter and learner ID; entries may start at any progress value, and
    /// the initial quorum-accepted value is computed from the initial voter
    /// progress.
    pub fn new(
        quorum_set: QS,
        learner_ids: impl IntoIterator<Item = Entry::Id>,
        mut default_entry: impl FnMut(Entry::Id) -> Entry,
    ) -> Self {
        let voter_ids = quorum_set.ids().collect::<BTreeSet<_>>();
        let learner_ids =
            learner_ids.into_iter().filter(|id| !voter_ids.contains(id)).collect::<BTreeSet<_>>();

        let mut entries = voter_ids.into_iter().map(&mut default_entry).collect::<Vec<_>>();

        let voter_count = entries.len();

        // Initial progress is not necessarily `Progress::default()`: sort voters
        // in descending progress order and find the greatest accepted value.
        entries.sort_by(|a, b| b.progress().cmp(a.progress()));

        let mut quorum_accepted = Entry::Progress::default();
        for i in 0..voter_count {
            let ids = entries[..=i].iter().map(|entry| entry.id());
            if quorum_set.is_quorum(ids) {
                quorum_accepted = entries[i].progress().clone();
                break;
            }
        }

        entries.extend(learner_ids.into_iter().map(default_entry));

        Self {
            quorum_set,
            quorum_accepted,
            voter_count,
            entries,
            stat: Default::default(),
        }
    }

    /// Find the index of the specified id.
    #[inline(always)]
    fn index(&self, target: &Entry::Id) -> Option<usize> {
        self.entries.iter().position(|item| item.id() == target)
    }

    /// Move an element at `index` up so that voters stay sorted.
    #[inline(always)]
    fn move_up(&mut self, index: usize) -> usize {
        self.stat.move_count += 1;
        for i in (0..index).rev() {
            if self.entries[i].progress() < self.entries[i + 1].progress() {
                self.entries.swap(i, i + 1);
            } else {
                return i + 1;
            }
        }

        0
    }

    /// Move a voter element at `index` down so that voters stay sorted
    /// after its progress value is lowered.
    ///
    /// It is the counterpart of [`Self::move_up`], used by [`Self::reset_entry_with()`].
    fn move_down(&mut self, index: usize) -> usize {
        self.stat.move_count += 1;
        let mut i = index;
        while i + 1 < self.voter_count
            && self.entries[i].progress() < self.entries[i + 1].progress()
        {
            self.entries.swap(i, i + 1);
            i += 1;
        }

        i
    }

    /// Return mutable entries without maintaining the progress ordering.
    ///
    /// Mutating progress values through this iterator can leave the internal
    /// ordering and quorum-accepted value stale. Normal progress updates must
    /// use [`Self::update_progress()`] or [`Self::update_entry_with()`] instead.
    /// Mutating entry IDs can corrupt membership lookup.
    pub fn iter_mut_without_reorder(&mut self) -> IterMut<'_, Entry> {
        self.entries.iter_mut()
    }

    #[cfg(test)]
    pub(crate) fn stat(&self) -> &ProgressStats {
        &self.stat
    }

    /// Return a display adapter that formats entries with a caller-provided formatter.
    pub fn display_with<Fmt>(&self, f: Fmt) -> DisplayVecProgress<'_, Entry, QS, Fmt>
    where Fmt: Fn(&mut Formatter<'_>, &Entry) -> std::fmt::Result {
        DisplayVecProgress { inner: self, f }
    }

    /// Validates progress-update invariants in debug builds.
    fn debug_assert_progress_valid(&self) {
        #[cfg(debug_assertions)]
        self.validate().expect("VecProgress progress invariant violation");
    }
}

impl<Entry, QS> VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    Entry::Id: Ord + Clone + Debug,
    Entry::Progress: Debug,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Update one progress value monotonically and recalculate the quorum-accepted value.
    ///
    /// It returns `None` if the `id` is not found.
    /// Otherwise, it returns the current quorum-accepted value.
    /// Updating with the same value leaves the state unchanged.
    ///
    /// # Algorithm
    ///
    /// Only one case can increase the quorum-accepted value: the **previous value**
    /// is less than or equal to the current quorum-accepted value, and the **new
    /// value** is greater than it.
    ///
    /// This avoids many unnecessary quorum recalculations and sorts. Progress
    /// entries above the quorum-accepted value are kept in descending order, and
    /// entries at or below it do not need to be sorted.
    ///
    /// E.g., given 3 ids with values `1,3,5`, as shown in the figure below:
    ///
    /// ```text
    /// a -----------+-------->
    /// b -------+------------>
    /// c ---+---------------->
    /// ------------------------------
    ///      1   3   5
    /// ```
    ///
    /// the quorum-accepted is `3` and assumes a majority quorum set is used.
    /// Then:
    /// - update_progress(a, 6): nothing to do: quorum-accepted is still 3;
    /// - update_progress(b, 4): re-calc:       quorum-accepted becomes 4;
    /// - update_progress(b, 6): re-calc:       quorum-accepted becomes 5;
    /// - update_progress(c, 2): nothing to do: quorum-accepted is still 3;
    /// - update_progress(c, 3): nothing to do: quorum-accepted is still 3;
    /// - update_progress(c, 4): re-calc:       quorum-accepted becomes 4;
    /// - update_progress(c, 6): re-calc:       quorum-accepted becomes 5;
    fn update_progress_with<F>(&mut self, id: &Entry::Id, f: F) -> Option<&Entry::Progress>
    where F: FnOnce(&mut Entry::Progress) {
        self.update_entry_with(id, |entry| f(entry.progress_mut()))
    }

    /// Update an entry and recalculate the quorum-accepted value.
    ///
    /// Use this when application-owned fields must change together with the
    /// progress value. The progress update must not lower progress, and the
    /// entry ID must not change.
    ///
    /// It returns `None` if the `id` is not found.
    /// Otherwise, it returns the current quorum-accepted value.
    pub fn update_entry_with<F>(&mut self, id: &Entry::Id, f: F) -> Option<&Entry::Progress>
    where F: FnOnce(&mut Entry) {
        self.stat.update_count += 1;

        let index = self.index(id)?;

        let prev_progress = self.entries[index].progress().clone();

        f(&mut self.entries[index]);

        debug_assert!(self.entries[index].id() == id);

        Some(self.update_at(index, prev_progress))
    }

    /// Update application-owned data without recalculating quorum-accepted progress.
    ///
    /// This method only exposes [`VecProgressEntryData::Data`], so it cannot
    /// change progress or invalidate the ordering maintained by [`VecProgress`].
    ///
    /// Returns the updated data when `id` is found, otherwise returns `None`.
    pub fn update_data_with<F>(&mut self, id: &Entry::Id, f: F) -> Option<&Entry::Data>
    where
        Entry: VecProgressEntryData,
        F: FnOnce(&mut Entry::Data),
    {
        let index = self.index(id)?;

        f(self.entries[index].data_mut());

        Some(self.entries[index].data())
    }

    /// Update an entry whose progress value may move backward, for example when
    /// replication progress is reset upon log reversion.
    ///
    /// If the progress value is lowered, the entry is moved down to keep the
    /// values greater than `quorum_accepted` sorted. The recorded
    /// quorum-accepted value is deliberately not recalculated: a value accepted
    /// by a quorum must never be withdrawn.
    /// The entry ID must not be changed.
    ///
    /// It returns the updated entry if the `id` is found, otherwise returns `None`.
    pub fn reset_entry_with<F>(&mut self, id: &Entry::Id, f: F) -> Option<&Entry>
    where F: FnOnce(&mut Entry) {
        let index = self.index(id)?;

        let prev_progress = self.entries[index].progress().clone();

        f(&mut self.entries[index]);

        debug_assert!(self.entries[index].id() == id);
        debug_assert!(self.entries[index].progress() <= &prev_progress);

        // Learners are never reordered.
        let new_index =
            if index < self.voter_count && self.entries[index].progress() < &prev_progress {
                self.move_down(index)
            } else {
                index
            };

        self.debug_assert_progress_valid();
        Some(&self.entries[new_index])
    }

    fn update_at(&mut self, index: usize, prev_progress: Entry::Progress) -> &Entry::Progress {
        debug_assert!(self.entries[index].progress() >= &prev_progress,);

        // No change, return early
        if &prev_progress == self.entries[index].progress() {
            self.debug_assert_progress_valid();
            return &self.quorum_accepted;
        }

        // Learner does not grant a value.
        // And it won't be moved up to adjust the order.
        if index >= self.voter_count {
            self.debug_assert_progress_valid();
            return &self.quorum_accepted;
        }

        let prev_le_qa = prev_progress <= self.quorum_accepted;
        let new_gt_qa = self.entries[index].progress() > &self.quorum_accepted;

        // Sort and find the greatest value accepted by a quorum set.

        if new_gt_qa {
            let new_index = self.move_up(index);

            if prev_le_qa {
                // From high to low, find the max value that has constituted a quorum.
                for i in new_index..self.voter_count {
                    let prog = self.entries[i].progress();

                    // No need to recalculate an already quorum-accepted value.
                    if prog <= &self.quorum_accepted {
                        break;
                    }

                    // Ids of the target that has value GE `entries[i]`
                    let it = self.entries[0..=i].iter().map(|item| item.id());

                    self.stat.is_quorum_count += 1;

                    if self.quorum_set.is_quorum(it) {
                        self.quorum_accepted = prog.clone();
                        break;
                    }
                }
            }
        }

        self.debug_assert_progress_valid();
        &self.quorum_accepted
    }

    /// Set one node's progress and recalculate the quorum-accepted value.
    ///
    /// The new value must be greater than or equal to the current progress. Use
    /// [`Self::reset_entry_with()`] for an explicit backward move.
    ///
    /// It returns `None` if the `id` is not found.
    /// Otherwise, it returns the current quorum-accepted value.
    pub fn update_progress(
        &mut self,
        id: &Entry::Id,
        value: Entry::Progress,
    ) -> Option<&Entry::Progress> {
        self.update_progress_with(id, |x| *x = value)
    }

    /// Increase one node's progress if `value` is greater than its current value.
    ///
    /// It returns `None` if the `id` is not found.
    /// Otherwise, it returns the current quorum-accepted value.
    pub fn increase_to(
        &mut self,
        id: &Entry::Id,
        value: Entry::Progress,
    ) -> Option<&Entry::Progress> {
        self.update_progress_with(id, |x| {
            if value > *x {
                *x = value;
            }
        })
    }

    /// Return the tracked entry for `id`.
    pub fn try_get(&self, id: &Entry::Id) -> Option<&Entry> {
        let index = self.index(id)?;
        Some(&self.entries[index])
    }

    /// Return the greatest progress value accepted by the quorum set.
    ///
    /// If no value has been accepted by any quorum, it returns
    /// `Progress::default()`.
    ///
    /// In Raft, this is the replication progress reached by enough voters to be
    /// considered committed once the term-specific commit rule also allows it.
    pub fn quorum_accepted(&self) -> &Entry::Progress {
        &self.quorum_accepted
    }

    /// Return the quorum set that decides which entries constitute a quorum.
    pub fn quorum_set(&self) -> &QS {
        &self.quorum_set
    }

    /// Return the number of voter entries.
    ///
    /// [`Self::iter()`] yields these voters first, before the learners.
    pub fn voter_count(&self) -> usize {
        self.voter_count
    }

    /// Iterate over all entries, with voters first and learners after them.
    pub fn iter(&self) -> Iter<'_, Entry> {
        self.entries.as_slice().iter()
    }

    /// Map every entry and collect the mapped values.
    pub fn collect_mapped<F, T, C>(&self, f: F) -> C
    where
        F: Fn(&Entry) -> T,
        C: FromIterator<T>,
    {
        self.iter().map(f).collect()
    }

    /// Build a tracker for a new quorum set while preserving progress for shared IDs.
    ///
    /// Entries whose IDs still exist in the new voter or learner set keep their
    /// previous progress and application data. New IDs are initialized through
    /// `default_entry`. The quorum-accepted value is recomputed for the new
    /// quorum set, so it may be lower than before the upgrade.
    pub fn upgrade_quorum_set(
        self,
        quorum_set: QS,
        learner_ids: impl IntoIterator<Item = Entry::Id>,
        mut default_entry: impl FnMut(Entry::Id) -> Entry,
    ) -> Self {
        let mut old = self
            .entries
            .into_iter()
            .map(|entry| (entry.id().clone(), entry))
            .collect::<BTreeMap<_, _>>();

        let mut new_prog = Self::new(quorum_set, learner_ids, |id| {
            old.remove(&id).unwrap_or_else(|| default_entry(id))
        });

        new_prog.stat = self.stat;
        new_prog
    }

    /// Return whether the given ID is a voter.
    ///
    /// A voter is a node in the quorum set that can grant a value.
    /// A learner's progress is also tracked, but it will never grant a value.
    ///
    /// If the given id is not in this [`VecProgress`], it returns `None`.
    pub fn is_voter(&self, id: &Entry::Id) -> Option<bool> {
        let index = self.index(id)?;
        Some(index < self.voter_count)
    }
}

impl<Entry, QS> IntoIterator for VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    QS: QuorumSet<Id = Entry::Id>,
{
    type Item = Entry;
    type IntoIter = std::vec::IntoIter<Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<Entry, QS> Validate for VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    Entry::Id: Ord + Clone + Debug,
    Entry::Progress: Debug,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Validates the voter-order invariant maintained after progress updates.
    fn validate(&self) -> Result<(), Box<dyn Error>> {
        self.validate_voter_order()
    }
}

impl<Entry, QS> VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    Entry::Id: Ord + Clone + Debug,
    Entry::Progress: Debug,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Validates that voter entries whose progress is greater than
    /// `quorum_accepted` form a descending prefix, reporting the first
    /// out-of-order entry with the current voter and learner progress state.
    fn validate_voter_order(&self) -> Result<(), Box<dyn Error>> {
        let voters = &self.entries[..self.voter_count];
        let progress_state = || {
            let voter_progress = voters
                .iter()
                .map(|entry| (entry.id().clone(), entry.progress().clone()))
                .collect::<Vec<_>>();
            let learner_progress = self.entries[self.voter_count..]
                .iter()
                .map(|entry| (entry.id().clone(), entry.progress().clone()))
                .collect::<Vec<_>>();

            (voter_progress, learner_progress)
        };

        let suffix_start = voters
            .iter()
            .position(|entry| entry.progress() <= &self.quorum_accepted)
            .unwrap_or(voters.len());

        for (previous_index, pair) in voters[..suffix_start].windows(2).enumerate() {
            let previous = &pair[0];
            let item = &pair[1];
            if previous.progress() < item.progress() {
                let (voter_progress, learner_progress) = progress_state();
                return Err(format!(
                    "voter progress above quorum_accepted is not descending: quorum_accepted={:?}, previous_entry={:?}, out_of_order_entry={:?}, voter_progress={voter_progress:?}, learner_progress={learner_progress:?}",
                    self.quorum_accepted,
                    (previous_index, previous.id(), previous.progress()),
                    (previous_index + 1, item.id(), item.progress())
                )
                .into());
            }
        }

        for (suffix_offset, item) in voters[suffix_start..].iter().enumerate() {
            if item.progress() <= &self.quorum_accepted {
                continue;
            }

            let index = suffix_start + suffix_offset;
            let (voter_progress, learner_progress) = progress_state();
            return Err(format!(
                "voter progress above quorum_accepted appears after the unsorted suffix: quorum_accepted={:?}, out_of_order_entry={:?}, voter_progress={voter_progress:?}, learner_progress={learner_progress:?}",
                self.quorum_accepted,
                (index, item.id(), item.progress())
            )
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod vec_progress_test;
