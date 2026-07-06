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
    QS: QuorumSet<Id = Entry::Id> + 'static,
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
    /// voters and never contribute to quorum acceptance. `default_entry` builds
    /// the initial entry for every voter and learner ID.
    pub fn new(
        quorum_set: QS,
        learner_ids: impl IntoIterator<Item = Entry::Id>,
        mut default_entry: impl FnMut(Entry::Id) -> Entry,
    ) -> Self {
        let mut entries = quorum_set.ids().map(&mut default_entry).collect::<Vec<_>>();

        let voter_count = entries.len();

        entries.extend(learner_ids.into_iter().map(default_entry));

        let this = Self {
            quorum_set,
            quorum_accepted: Default::default(),
            voter_count,
            entries,
            stat: Default::default(),
        };
        this.validate_initial_state().expect("VecProgress construction invariant violation");
        this
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
    pub fn get(&self, id: &Entry::Id) -> Option<&Entry> {
        let index = self.index(id)?;
        Some(&self.entries[index])
    }

    /// Return the greatest progress value accepted by the quorum set.
    ///
    /// In Raft, this is the replication progress reached by enough voters to be
    /// considered committed once the term-specific commit rule also allows it.
    pub fn quorum_accepted(&self) -> &Entry::Progress {
        &self.quorum_accepted
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
    /// `default_entry`.
    pub fn upgrade_quorum_set(
        self,
        quorum_set: QS,
        learner_ids: impl IntoIterator<Item = Entry::Id>,
        default_entry: impl FnMut(Entry::Id) -> Entry,
    ) -> Self {
        let mut new_prog = Self::new(quorum_set, learner_ids, default_entry);

        new_prog.stat = self.stat.clone();

        for item in self.into_iter() {
            new_prog.replace(item);
        }
        new_prog.debug_assert_progress_valid();
        new_prog
    }

    /// Replace the entry for the same ID and update quorum-accepted progress.
    fn replace(&mut self, entry: Entry) -> Option<&Entry::Progress> {
        self.stat.update_count += 1;

        let index = self.index(entry.id())?;

        let prev_progress = self.entries[index].progress().clone();

        self.entries[index] = entry;

        Some(self.update_at(index, prev_progress))
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
        self.validate_progress()
    }
}

impl<Entry, QS> VecProgress<Entry, QS>
where
    Entry: VecProgressEntry,
    Entry::Id: Ord + Clone + Debug,
    Entry::Progress: Debug,
    QS: QuorumSet<Id = Entry::Id>,
{
    /// Validates all construction-time invariants for a new [`VecProgress`].
    fn validate_initial_state(&self) -> Result<(), Box<dyn Error>> {
        self.validate_voter_count()?;
        self.validate_unique_entry_ids()?;
        self.validate_quorum_membership()?;
        self.validate_progress()?;

        Ok(())
    }

    /// Validates the voter-order invariant affected by progress updates.
    fn validate_progress(&self) -> Result<(), Box<dyn Error>> {
        self.validate_voter_order()
    }

    /// Validates that `voter_count` partitions `entries` into a valid voter
    /// prefix and learner suffix.
    fn validate_voter_count(&self) -> Result<(), Box<dyn Error>> {
        if self.voter_count > self.entries.len() {
            return Err(invalid(format!(
                "voter_count {} exceeds entry count {}",
                self.voter_count,
                self.entries.len()
            )));
        }

        Ok(())
    }

    /// Validates that every tracked entry ID is unique across both voters and
    /// learners.
    fn validate_unique_entry_ids(&self) -> Result<(), Box<dyn Error>> {
        let mut positions = BTreeMap::<Entry::Id, Vec<usize>>::new();
        for (i, entry) in self.entries.iter().enumerate() {
            positions.entry(entry.id().clone()).or_default().push(i);
        }

        let duplicates = positions
            .into_iter()
            .filter(|(_, positions)| positions.len() > 1)
            .collect::<BTreeMap<_, _>>();

        if !duplicates.is_empty() {
            return Err(invalid(format!("duplicate entry ids: {duplicates:?}")));
        }

        Ok(())
    }

    /// Validates that the quorum-set IDs exactly match the voter-entry IDs and
    /// do not appear in the learner suffix.
    fn validate_quorum_membership(&self) -> Result<(), Box<dyn Error>> {
        let quorum_ids = self.quorum_set.ids().collect::<BTreeSet<_>>();
        let voter_entry_ids = self.entries[..self.voter_count]
            .iter()
            .map(|entry| entry.id().clone())
            .collect::<BTreeSet<_>>();
        let learner_entry_ids = self.entries[self.voter_count..]
            .iter()
            .map(|entry| entry.id().clone())
            .collect::<BTreeSet<_>>();

        let missing_voter_ids =
            quorum_ids.difference(&voter_entry_ids).cloned().collect::<BTreeSet<_>>();
        let extra_voter_ids =
            voter_entry_ids.difference(&quorum_ids).cloned().collect::<BTreeSet<_>>();
        let learner_voter_ids =
            learner_entry_ids.intersection(&quorum_ids).cloned().collect::<BTreeSet<_>>();

        if quorum_ids.len() == self.voter_count
            && missing_voter_ids.is_empty()
            && extra_voter_ids.is_empty()
            && learner_voter_ids.is_empty()
        {
            return Ok(());
        }

        Err(invalid(format!(
            "quorum membership mismatch: quorum_ids={quorum_ids:?}, voter_entry_ids={voter_entry_ids:?}, learner_entry_ids={learner_entry_ids:?}, missing_voter_ids={missing_voter_ids:?}, extra_voter_ids={extra_voter_ids:?}, learner_voter_ids={learner_voter_ids:?}, voter_count={}",
            self.voter_count
        )))
    }

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
                return Err(invalid(format!(
                    "voter progress above quorum_accepted is not descending: quorum_accepted={:?}, previous_entry={:?}, out_of_order_entry={:?}, voter_progress={voter_progress:?}, learner_progress={learner_progress:?}",
                    self.quorum_accepted,
                    (previous_index, previous.id(), previous.progress()),
                    (previous_index + 1, item.id(), item.progress())
                )));
            }
        }

        for (suffix_offset, item) in voters[suffix_start..].iter().enumerate() {
            if item.progress() <= &self.quorum_accepted {
                continue;
            }

            let index = suffix_start + suffix_offset;
            let (voter_progress, learner_progress) = progress_state();
            return Err(invalid(format!(
                "voter progress above quorum_accepted appears after the unsorted suffix: quorum_accepted={:?}, out_of_order_entry={:?}, voter_progress={voter_progress:?}, learner_progress={learner_progress:?}",
                self.quorum_accepted,
                (index, item.id(), item.progress())
            )));
        }

        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;

    use maplit::btreeset;
    use validit::Validate;

    use super::VecProgress;
    use crate::Node;
    use crate::QuorumTree;
    use crate::progress::VecProgressEntry;
    use crate::progress::VecProgressEntryData;
    use crate::quorum::QuorumSet;

    const LCG_A: u64 = 6364136223846793005;
    const LCG_C: u64 = 1442695040888963407;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct IdValData<ID, Val, Data> {
        id: ID,
        val: Val,
        data: Data,
    }

    impl<ID, Val, Data> IdValData<ID, Val, Data> {
        fn new(id: ID, val: Val, data: Data) -> Self {
            Self { id, val, data }
        }
    }

    impl<ID, Val, Data> VecProgressEntry for IdValData<ID, Val, Data>
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

    impl<ID, Val, Data> VecProgressEntryData for IdValData<ID, Val, Data>
    where
        ID: 'static + PartialEq,
        Val: Clone + Default + Ord,
    {
        type Data = Data;

        fn data(&self) -> &Self::Data {
            &self.data
        }

        fn data_mut(&mut self) -> &mut Self::Data {
            &mut self.data
        }
    }

    #[derive(Clone, Debug)]
    struct RequiredSetQuorum {
        ids: BTreeSet<u64>,
        required: BTreeSet<u64>,
    }

    impl RequiredSetQuorum {
        /// Build a quorum set that grants only when every required ID is present.
        ///
        /// This intentionally does not implement majority semantics, so it can
        /// catch accidental assumptions in `VecProgress`.
        fn new(
            ids: impl IntoIterator<Item = u64>,
            required: impl IntoIterator<Item = u64>,
        ) -> Self {
            Self {
                ids: ids.into_iter().collect(),
                required: required.into_iter().collect(),
            }
        }
    }

    impl QuorumSet for RequiredSetQuorum {
        type Id = u64;

        type Iter = std::collections::btree_set::IntoIter<u64>;

        fn is_quorum<'a, I: Iterator<Item = &'a Self::Id> + Clone>(&self, ids: I) -> bool {
            let granted = ids.copied().collect::<BTreeSet<_>>();
            self.required.is_subset(&granted)
        }

        fn ids(&self) -> Self::Iter {
            self.ids.clone().into_iter()
        }
    }

    /// Advance a deterministic pseudo-random sequence for model-based tests.
    ///
    /// The generator is intentionally tiny and reproducible; it is not used for
    /// randomness quality, only for broadening the monotonic update cases.
    fn next_random(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(LCG_A).wrapping_add(LCG_C);
        *seed
    }

    /// Build learner IDs as the complement of the current quorum IDs.
    ///
    /// Randomized upgrade tests use this to preserve progress for all known
    /// nodes when switching between simple, joint, and shrunk quorum sets.
    fn learner_ids_for<QS>(quorum_set: &QS, known_ids: impl IntoIterator<Item = u64>) -> Vec<u64>
    where QS: QuorumSet<Id = u64> {
        let voter_ids = quorum_set.ids().collect::<BTreeSet<_>>();
        known_ids.into_iter().filter(|id| !voter_ids.contains(id)).collect()
    }

    /// Copy the `Option<&u64>` returned by progress updates.
    fn copy_option(res: Option<&u64>) -> Option<u64> {
        res.copied()
    }

    /// Compute quorum-accepted progress with a straightforward reference model.
    ///
    /// This intentionally ignores `VecProgress`'s internal ordering optimization:
    /// it tries candidate progress values from high to low and returns the first
    /// value whose reached node IDs form a quorum.
    fn model_quorum_accepted<QS>(quorum_set: &QS, entries: &[(u64, u64)]) -> u64
    where QS: QuorumSet<Id = u64> {
        let values = entries.iter().map(|item| (item.0, item.1)).collect::<BTreeMap<_, _>>();
        let mut candidates = quorum_set.ids().map(|id| values[&id]).collect::<Vec<_>>();

        candidates.sort_unstable_by(|a, b| b.cmp(a));
        candidates.dedup();

        for candidate in candidates {
            let ids = values.iter().filter_map(|(id, val)| (*val >= candidate).then_some(id));
            if quorum_set.is_quorum(ids) {
                return candidate;
            }
        }

        0
    }

    /// Assert that `VecProgress` agrees with the reference model.
    ///
    /// This checks both the externally visible quorum-accepted value and the
    /// internal ordering invariant relied on by the optimized update algorithm.
    fn assert_matches_model<QS>(progress: &VecProgress<(u64, u64), QS>, context: &str)
    where QS: QuorumSet<Id = u64> {
        let want = model_quorum_accepted(&progress.quorum_set, &progress.entries);
        assert_eq!(
            &want,
            progress.quorum_accepted(),
            "{}: entries: {:?}",
            context,
            progress.entries
        );
        assert_voter_prefix_is_sorted(progress, context);
    }

    /// Assert the voter ordering invariant maintained by `update_progress()`.
    ///
    /// Voter entries above `quorum_accepted` must form a descending prefix.
    /// Learners are excluded because learner progress does not grant quorum.
    fn assert_voter_prefix_is_sorted<QS>(progress: &VecProgress<(u64, u64), QS>, context: &str)
    where QS: QuorumSet<Id = u64> {
        let quorum_accepted = *progress.quorum_accepted();
        let mut previous = None;
        let mut seen_unsorted_suffix = false;

        for item in &progress.entries[..progress.voter_count] {
            if item.1 <= quorum_accepted {
                seen_unsorted_suffix = true;
                continue;
            }

            assert!(
                !seen_unsorted_suffix,
                "{}: non-prefix above-quorum entry: {:?}",
                context, progress.entries
            );
            if let Some(prev) = previous {
                assert!(
                    prev >= item.1,
                    "{}: unsorted voters: {:?}",
                    context,
                    progress.entries
                );
            }
            previous = Some(item.1);
        }
    }

    fn assert_initial_invalid_contains<QS>(progress: &VecProgress<(u64, u64), QS>, want: &str)
    where QS: QuorumSet<Id = u64> {
        let err = progress.validate_initial_state().unwrap_err().to_string();
        assert!(err.contains(want), "error: {err}");
    }

    fn assert_err_contains(err: Box<dyn std::error::Error>, want: &str) {
        let err = err.to_string();
        assert!(err.contains(want), "error: {err}");
    }

    #[test]
    fn vec_progress_new() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6, 7], |id| (id, 0));

        assert_eq!(
            vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (6, 0), (7, 0),],
            progress.entries
        );
        assert_eq!(5, progress.voter_count);
    }

    #[test]
    fn vec_progress_new_with_quorum_tree() {
        let group_a = QuorumTree::new(2, [Node::Id(1), Node::Id(2), Node::Id(3)]).unwrap();
        let group_b = QuorumTree::new(2, [Node::Id(4), Node::Id(5), Node::Id(6)]).unwrap();
        let quorum_set =
            QuorumTree::new(2, [Node::Subtree(group_a), Node::Subtree(group_b)]).unwrap();
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

        assert_eq!(
            vec![(1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 0)],
            progress.entries
        );
        assert_eq!(6, progress.voter_count);

        assert_eq!(Some(&0), progress.update_progress(&1, 10));
        assert_eq!(Some(&0), progress.update_progress(&2, 10));
        assert_eq!(Some(&0), progress.update_progress(&4, 10));
        assert_eq!(Some(&10), progress.update_progress(&5, 10));
    }

    #[test]
    fn vec_progress_validate_rejects_duplicate_entry_ids() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.entries[3].0 = 1;

        assert!(progress.validate().is_ok());
        assert_initial_invalid_contains(&progress, "duplicate entry id");
        assert_initial_invalid_contains(&progress, "1: [1, 3]");
    }

    #[test]
    fn vec_progress_validate_reports_membership_mismatches() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3, 4], |id| (id, 0));

        progress.entries[0].0 = 9;
        progress.entries[3].0 = 2;

        let err = progress.validate_quorum_membership().unwrap_err();
        assert_err_contains(err, "missing_voter_ids={0}");

        let err = progress.validate_quorum_membership().unwrap_err();
        assert_err_contains(err, "extra_voter_ids={9}");

        let err = progress.validate_quorum_membership().unwrap_err();
        assert_err_contains(err, "learner_voter_ids={2}");
    }

    #[test]
    fn vec_progress_validate_reports_voter_order_mismatches() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.quorum_accepted = 4;
        progress.entries[0].1 = 7;
        progress.entries[1].1 = 3;
        progress.entries[2].1 = 6;
        progress.entries[3].1 = 9;

        let err = progress.validate_voter_order().unwrap_err().to_string();
        assert!(
            err.contains("appears after the unsorted suffix"),
            "error: {err}"
        );
        assert!(err.contains("out_of_order_entry=(2, 2, 6)"), "error: {err}");
        assert!(
            err.contains("voter_progress=[(0, 7), (1, 3), (2, 6)]"),
            "error: {err}"
        );
        assert!(err.contains("learner_progress=[(3, 9)]"), "error: {err}");

        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.quorum_accepted = 4;
        progress.entries[0].1 = 5;
        progress.entries[1].1 = 6;
        progress.entries[3].1 = 9;

        let err = progress.validate_voter_order().unwrap_err().to_string();
        assert!(err.contains("previous_entry=(0, 0, 5)"), "error: {err}");
        assert!(err.contains("out_of_order_entry=(1, 1, 6)"), "error: {err}");
        assert!(
            err.contains("voter_progress=[(0, 5), (1, 6), (2, 0)]"),
            "error: {err}"
        );
        assert!(err.contains("learner_progress=[(3, 9)]"), "error: {err}");
    }

    #[test]
    fn vec_progress_validate_accepts_reset_below_quorum_accepted() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

        progress.update_progress(&0, 10).unwrap();
        progress.update_progress(&1, 10).unwrap();
        progress.reset_entry_with(&0, |entry| entry.1 = 0).unwrap();

        assert!(progress.validate().is_ok());
    }

    #[test]
    fn vec_progress_tuple_entry() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        assert_eq!(
            vec![(0, 0), (1, 0), (2, 0), (3, 0)],
            progress.iter().cloned().collect::<Vec<_>>()
        );
        assert_eq!(Some(&0), progress.update_progress(&0, 5));
        assert_eq!(Some(&5), progress.update_progress(&1, 5));
        assert_eq!(Some(&(0, 5)), progress.get(&0));
        assert_eq!(Some(&(1, 5)), progress.get(&1));
        assert_eq!(&5, progress.quorum_accepted());
    }

    #[test]
    fn vec_progress_index() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6, 7], |id| (id, 0));

        assert_eq!(Some(0), progress.index(&0));
        assert_eq!(Some(1), progress.index(&1));
        assert_eq!(Some(4), progress.index(&4));
        assert_eq!(Some(5), progress.index(&6));
        assert_eq!(Some(6), progress.index(&7));
        assert_eq!(None, progress.index(&9));
        assert_eq!(None, progress.index(&100));
    }

    #[test]
    fn vec_progress_get() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6, 7], |id| (id, 0));

        progress.update_progress(&6, 5);
        assert_eq!(Some(&(6, 5)), progress.get(&6));
        assert_eq!(Some(&5), progress.get(&6).map(|x| &x.1));
        assert_eq!(None, progress.get(&9));

        progress.update_progress(&6, 10);
        assert_eq!(Some(&10), progress.get(&6).map(|x| &x.1));
    }

    #[test]
    fn vec_progress_iter() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6, 7], |id| (id, 0));

        progress.update_progress(&7, 7);
        progress.update_progress(&3, 3);
        progress.update_progress(&1, 1);

        assert_eq!(
            vec![(3, 3), (1, 1), (0, 0), (2, 0), (4, 0), (6, 0), (7, 7),],
            progress.iter().cloned().collect::<Vec<_>>(),
            "iter() returns voter first, followed by learners"
        );
    }

    #[test]
    fn vec_progress_move_up() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6], |id| (id, 0));

        // initial: 0-0, 1-0, 2-0, 3-0, 4-0
        let cases = [
            (
                (1, 2),
                vec![(1, 2), (0, 0), (2, 0), (3, 0), (4, 0), (6, 0)],
                0,
            ),
            (
                (2, 3),
                vec![(2, 3), (1, 2), (0, 0), (3, 0), (4, 0), (6, 0)],
                0,
            ),
            (
                (1, 3),
                vec![(2, 3), (1, 3), (0, 0), (3, 0), (4, 0), (6, 0)],
                1,
            ), // no move
            (
                (4, 8),
                vec![(4, 8), (2, 3), (1, 3), (0, 0), (3, 0), (6, 0)],
                0,
            ),
            (
                (0, 5),
                vec![(4, 8), (0, 5), (2, 3), (1, 3), (3, 0), (6, 0)],
                1,
            ), // move to 1st
        ];
        for (ith, ((id, v), want_vec, want_new_index)) in cases.iter().enumerate() {
            // Update a value and move it up to keep the order.
            let index = progress.index(id).unwrap();
            progress.entries[index].1 = *v;
            let got = progress.move_up(index);

            assert_eq!(
                want_vec, &progress.entries,
                "{}-th case: idx:{}, v:{}",
                ith, *id, *v
            );
            assert_eq!(
                *want_new_index, got,
                "{}-th case: idx:{}, v:{}",
                ith, *id, *v
            );
        }
    }

    #[test]
    fn vec_progress_update_progress() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6], |id| (id, 0));

        // initial: 0,0,0,0,0
        let cases = vec![
            ((6, 9), Some(&0)), // 0,0,0,0,0,9 // learner won't affect quorum-accepted
            ((1, 2), Some(&0)), // 0,2,0,0,0,0
            ((2, 3), Some(&0)), // 0,2,3,0,0,0
            ((3, 1), Some(&1)), // 0,2,3,1,0,0
            ((4, 5), Some(&2)), // 0,2,3,1,5,0
            ((0, 4), Some(&3)), // 4,2,3,1,5,0
            ((3, 2), Some(&3)), // 4,2,3,2,5,0
            ((3, 3), Some(&3)), // 4,2,3,2,5,0
            ((1, 4), Some(&4)), // 4,4,3,2,5,0
            ((9, 1), None),     // nonexistent id, ignore.
        ];

        for (ith, ((id, v), want_quorum_accepted)) in cases.iter().enumerate() {
            let got = progress.update_progress_with(id, |x| *x = *v);
            assert_eq!(
                want_quorum_accepted.clone(),
                got,
                "{}-th case: id:{}, v:{}",
                ith,
                id,
                v
            );
        }
    }

    #[test]
    fn vec_progress_matches_reference_model_for_monotonic_updates() {
        let cases = [
            (vec![btreeset! {0, 1, 2, 3, 4}], vec![5, 6]),
            (vec![btreeset! {0, 1, 2}, btreeset! {2, 3, 4}], vec![5, 6]),
        ];

        for (case_id, (quorum_set, learners)) in cases.into_iter().enumerate() {
            for seed in 0..32 {
                let mut seed = seed + 1;
                let mut progress =
                    VecProgress::<(u64, u64), _>::new(quorum_set.clone(), learners.clone(), |id| {
                        (id, 0)
                    });

                assert_matches_model(&progress, &format!("case-{case_id} seed-{seed} initial"));

                for step in 0..128 {
                    let id = next_random(&mut seed) % 8;
                    let value = progress.get(&id).map(|entry| entry.1).unwrap_or_default()
                        + next_random(&mut seed) % 7
                        + 1;
                    let got = copy_option(progress.update_progress(&id, value));
                    let want = model_quorum_accepted(&progress.quorum_set, &progress.entries);
                    let want_result = progress.get(&id).map(|_| want);
                    let context =
                        format!("case-{case_id} seed-{seed} step-{step} update-{id}-{value}");

                    assert_eq!(
                        want_result, got,
                        "{context}: entries: {:?}",
                        progress.entries
                    );
                    assert_matches_model(&progress, &context);
                }
            }
        }
    }

    #[test]
    fn vec_progress_matches_reference_model_after_random_quorum_upgrades() {
        let quorum_sets = [
            vec![btreeset! {0, 1, 2, 3, 4}],
            vec![btreeset! {0, 1, 2}, btreeset! {2, 3, 4}],
            vec![btreeset! {2, 3, 4}],
            vec![btreeset! {1, 3, 5}, btreeset! {3, 4, 5, 6}],
            vec![btreeset! {0, 5, 6}],
        ];
        let known_ids = (0..=8).collect::<Vec<_>>();

        for seed in 0..16 {
            let mut seed = seed + 11;
            let quorum_set = quorum_sets[0].clone();
            let learner_ids = learner_ids_for(&quorum_set, known_ids.clone());
            let mut progress =
                VecProgress::<(u64, u64), _>::new(quorum_set, learner_ids, |id| (id, 0));

            assert_matches_model(&progress, &format!("seed-{seed} initial"));

            for round in 0..24 {
                for step in 0..16 {
                    let id = next_random(&mut seed) % 10;
                    let value = progress.get(&id).map(|entry| entry.1).unwrap_or_default()
                        + next_random(&mut seed) % 11
                        + 1;
                    progress.update_progress(&id, value);
                    assert_matches_model(
                        &progress,
                        &format!("seed-{seed} round-{round} step-{step} update"),
                    );
                }

                let quorum_index = next_random(&mut seed) as usize % quorum_sets.len();
                let quorum_set = quorum_sets[quorum_index].clone();
                let learner_ids = learner_ids_for(&quorum_set, known_ids.clone());

                progress = progress.upgrade_quorum_set(quorum_set, learner_ids, |id| (id, 0));
                assert_matches_model(
                    &progress,
                    &format!("seed-{seed} round-{round} upgrade-{quorum_index}"),
                );
            }
        }
    }

    #[test]
    fn vec_progress_joint_quorum_update_progress() {
        let quorum_set = vec![btreeset! {0, 1, 2}, btreeset! {2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [5, 6], |id| (id, 0));

        let cases = [
            (0, 5, 0),
            (1, 5, 0),
            (2, 4, 0),
            (3, 4, 4),
            (4, 6, 4),
            (3, 6, 5),
            (2, 7, 5),
            (0, 7, 6),
        ];

        for (ith, (id, value, want_quorum_accepted)) in cases.iter().enumerate() {
            let got = copy_option(progress.update_progress(id, *value));
            let context = format!("{ith}-th case: id:{id}, value:{value}");

            assert_eq!(
                Some(*want_quorum_accepted),
                got,
                "{context}: entries: {:?}",
                progress.entries
            );
            assert_matches_model(&progress, &context);
        }

        let entries: Vec<_> = progress.collect_mapped(|item| (item.0, item.1));
        assert_eq!(
            vec![(2, 7), (0, 7), (4, 6), (3, 6), (1, 5), (5, 0), (6, 0)],
            entries
        );
    }

    #[test]
    fn vec_progress_non_member_and_learner_edge_cases() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        assert!(
            std::panic::catch_unwind(|| {
                VecProgress::<(u64, u64), _>::new(quorum_set.clone(), [1, 3, 3], |id| (id, 0))
            })
            .is_err(),
            "duplicate IDs are invalid"
        );

        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        assert_eq!(vec![(0, 0), (1, 0), (2, 0), (3, 0)], progress.entries);
        assert_eq!(3, progress.voter_count);
        assert_eq!(Some(true), progress.is_voter(&1));
        assert_eq!(Some(false), progress.is_voter(&3));
        assert_eq!(None, progress.is_voter(&9));

        assert_eq!(Some(0), copy_option(progress.update_progress(&3, 7)));
        assert_eq!(vec![(0, 0), (1, 0), (2, 0), (3, 7)], progress.entries);

        assert_eq!(Some(0), copy_option(progress.update_progress(&1, 5)));
        assert_eq!(vec![(1, 5), (0, 0), (2, 0), (3, 7)], progress.entries);

        assert_eq!(Some(4), copy_option(progress.update_progress(&2, 4)));
        assert_eq!(vec![(1, 5), (2, 4), (0, 0), (3, 7)], progress.entries);

        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut no_learners = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

        assert_eq!(vec![(0, 0), (1, 0), (2, 0)], no_learners.entries);
        assert_eq!(3, no_learners.voter_count);
        assert_eq!(None, copy_option(no_learners.update_progress(&9, 5)));
        assert_eq!(vec![(0, 0), (1, 0), (2, 0)], no_learners.entries);
    }

    #[test]
    fn vec_progress_custom_quorum_set() {
        let quorum_set = RequiredSetQuorum::new([0, 1, 2, 3], [0, 3]);
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

        assert_eq!(Some(0), copy_option(progress.update_progress(&1, 10)));
        assert_eq!(Some(0), copy_option(progress.update_progress(&2, 9)));
        assert_eq!(Some(0), copy_option(progress.update_progress(&0, 8)));
        assert_matches_model(&progress, "custom quorum before required set is reached");

        assert_eq!(vec![(1, 10), (2, 9), (0, 8), (3, 0)], progress.entries);

        assert_eq!(Some(7), copy_option(progress.update_progress(&3, 7)));
        assert_eq!(&7, progress.quorum_accepted());
        assert_matches_model(&progress, "custom quorum reaches required set");

        assert_eq!(Some(8), copy_option(progress.update_progress(&3, 11)));
        assert_eq!(vec![(3, 11), (1, 10), (2, 9), (0, 8)], progress.entries);
        assert_matches_model(&progress, "custom quorum follows required set threshold");
    }

    #[test]
    fn vec_progress_update_progress_with() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6], |id| (id, 0));

        // Test that update_progress_with can use closures to modify values
        // Case 0: 0,2,0,0,0,0
        let got = progress.update_progress_with(&1, |x| *x += 2);
        assert_eq!(Some(&0), got, "case 0: id:1, +=2");

        // Case 1: 0,2,3,0,0,0
        let got = progress.update_progress_with(&2, |x| *x += 3);
        assert_eq!(Some(&0), got, "case 1: id:2, +=3");

        // Case 2: 0,2,3,1,0,0
        let got = progress.update_progress_with(&3, |x| *x = 1);
        assert_eq!(Some(&1), got, "case 2: id:3, =1");

        // Case 3: 0,2,3,1,5,0
        let got = progress.update_progress_with(&4, |x| *x += 5);
        assert_eq!(Some(&2), got, "case 3: id:4, +5");

        // Case 4: 4,2,3,1,5,0 - closure can see updated value
        let got = progress.update_progress_with(&0, |x| {
            *x += 4;
            assert_eq!(4, *x, "closure sees the updated value");
        });
        assert_eq!(Some(&3), got, "case 4: id:0, +=4");

        // Case 5: 4,2,3,2,5,0 - using max
        let got = progress.update_progress_with(&3, |x| *x = (*x).max(2));
        assert_eq!(Some(&3), got, "case 5: id:3, max(2)");

        // Case 6: 4,4,3,2,5,0
        let got = progress.update_progress_with(&1, |x| *x *= 2);
        assert_eq!(Some(&4), got, "case 6: id:1, *=2");

        // Verify final values
        assert_eq!(Some(&(0, 4)), progress.get(&0));
        assert_eq!(Some(&(1, 4)), progress.get(&1));
        assert_eq!(Some(&(2, 3)), progress.get(&2));
        assert_eq!(Some(&(3, 2)), progress.get(&3));
        assert_eq!(Some(&(4, 5)), progress.get(&4));
        assert_eq!(Some(&(6, 0)), progress.get(&6));

        // Test nonexistent id returns None
        let got = progress.update_progress_with(&9, |x| *x = 10);
        assert_eq!(None, got, "nonexistent id returns None");
    }

    #[test]
    fn vec_progress_update_data_with() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress =
            VecProgress::<IdValData<u64, u64, &'static str>, _>::new(quorum_set, [3], |id| {
                IdValData::new(id, 0, "foo")
            });

        assert_eq!(Some(&0), progress.update_progress(&1, 2));

        let stats_before = (
            progress.stat().update_count,
            progress.stat().move_count,
            progress.stat().is_quorum_count,
        );

        assert_eq!(
            Some(&"bar"),
            progress.update_data_with(&1, |data| *data = "bar")
        );
        assert_eq!(
            None,
            progress.update_data_with(&9, |data| *data = "unknown")
        );

        assert_eq!(
            vec![
                IdValData::new(1, 2, "bar"),
                IdValData::new(0, 0, "foo"),
                IdValData::new(2, 0, "foo"),
                IdValData::new(3, 0, "foo"),
            ],
            progress.iter().cloned().collect::<Vec<_>>()
        );
        assert_eq!(&0, progress.quorum_accepted());
        assert_eq!(
            stats_before,
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );
    }

    #[test]
    fn vec_progress_update_does_not_move_learner_elt() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6], |id| (id, 0));

        assert_eq!(Some(5), progress.index(&6));

        progress.update_progress(&6, 6);
        assert_eq!(Some(5), progress.index(&6), "learner is not moved");

        progress.update_progress(&4, 4);
        assert_eq!(Some(0), progress.index(&4), "voter is not moved");
    }

    #[test]
    fn vec_progress_upgrade_quorum_set() {
        let qs012 = vec![btreeset! {0, 1, 2}];
        let qs012_345 = vec![btreeset! {0, 1, 2}, btreeset! {3, 4, 5}];
        let qs345 = vec![btreeset! {3, 4, 5}];

        // Initially, quorum-accepted is 5

        let mut p012 = VecProgress::<(u64, u64), _>::new(qs012, [5], |id| (id, 0));

        p012.update_progress(&0, 5);
        p012.update_progress(&1, 6);
        p012.update_progress(&5, 9);
        assert_eq!(&5, p012.quorum_accepted());

        // After upgrading to a bigger quorum set, quorum-accepted fall back to 0

        let mut p012_345 = p012.upgrade_quorum_set(qs012_345, [6], |id| (id, 0));
        assert_eq!(
            &0,
            p012_345.quorum_accepted(),
            "quorum extended from 012 to 012_345, quorum-accepted falls back"
        );
        assert_eq!(Some(&(5, 9)), p012_345.get(&5), "inherit learner progress");

        // When quorum set shrinks, quorum-accepted becomes greater.

        p012_345.update_progress(&3, 7);
        p012_345.update_progress(&4, 8);
        assert_eq!(&5, p012_345.quorum_accepted());

        let p345 = p012_345.upgrade_quorum_set(qs345, [1], |id| (id, 0));

        assert_eq!(
            &8,
            p345.quorum_accepted(),
            "shrink quorum set, greater value becomes quorum-accepted"
        );
        assert_eq!(Some(&(1, 6)), p345.get(&1), "inherit voter progress");
    }

    #[test]
    fn vec_progress_upgrade_joint_quorum_set() {
        let qs01234 = vec![btreeset! {0, 1, 2, 3, 4}];
        let qs012_234 = vec![btreeset! {0, 1, 2}, btreeset! {2, 3, 4}];
        let qs345 = vec![btreeset! {3, 4, 5}];

        let mut p = VecProgress::<(u64, u64), _>::new(qs01234, [5], |id| (id, 0));

        for (id, value) in [(0, 9), (1, 8), (2, 7), (3, 2), (4, 1), (5, 10)] {
            p.update_progress(&id, value);
        }

        assert_eq!(&7, p.quorum_accepted());

        let mut joint = p.upgrade_quorum_set(qs012_234, [5, 6], |id| (id, 0));

        assert_eq!(
            &2,
            joint.quorum_accepted(),
            "joint quorum lowers the accepted value"
        );
        let entries: Vec<_> = joint.collect_mapped(|item| (item.0, item.1));
        assert_eq!(
            vec![(0, 9), (1, 8), (2, 7), (3, 2), (4, 1), (5, 10), (6, 0)],
            entries
        );
        assert_matches_model(&joint, "after upgrade to joint quorum");

        joint.update_progress(&3, 8);
        joint.update_progress(&4, 8);
        assert_eq!(&8, joint.quorum_accepted());
        assert_matches_model(&joint, "after joint quorum catches up");

        let shrunk = joint.upgrade_quorum_set(qs345, [0], |id| (id, 0));

        assert_eq!(&8, shrunk.quorum_accepted());
        let entries: Vec<_> = shrunk.collect_mapped(|item| (item.0, item.1));
        assert_eq!(vec![(5, 10), (3, 8), (4, 8), (0, 9)], entries);
        assert_matches_model(&shrunk, "after shrinking joint quorum");
    }

    #[test]
    fn vec_progress_is_voter() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6, 7], |id| (id, 0));

        assert_eq!(Some(true), progress.is_voter(&1));
        assert_eq!(Some(true), progress.is_voter(&3));
        assert_eq!(Some(false), progress.is_voter(&7));
        assert_eq!(None, progress.is_voter(&8));
    }

    #[test]
    fn vec_progress_display() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.update_progress(&1, 5);
        progress.update_progress(&2, 3);

        let display = format!(
            "{}",
            progress.display_with(|f, item| write!(f, "{}: {}", item.0, item.1))
        );
        assert_eq!("{1: 5, 2: 3, 0: 0, 3: 0}", display);
    }

    #[test]
    fn vec_progress_iter_mut_without_reorder() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        // Mutate values through iter_mut_without_reorder
        for item in progress.iter_mut_without_reorder() {
            if item.0 == 1 {
                item.1 = 10;
            }
        }

        assert_eq!(Some(&(1, 10)), progress.get(&1));
        assert_eq!(Some(&(0, 0)), progress.get(&0));
        assert_eq!(Some(&(2, 0)), progress.get(&2));
    }

    #[test]
    fn vec_progress_stat() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        assert_eq!(
            (0, 0, 0),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );

        progress.update_progress(&3, 10);
        assert_eq!(
            (1, 0, 0),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );

        progress.update_progress(&1, 5);
        assert_eq!(
            (2, 1, 1),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );

        progress.update_progress(&2, 4);
        assert_eq!(
            (3, 2, 2),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );

        progress.update_progress(&1, 6);
        assert_eq!(
            (4, 3, 2),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );

        progress.update_progress(&9, 7);
        assert_eq!(
            (5, 3, 2),
            (
                progress.stat().update_count,
                progress.stat().move_count,
                progress.stat().is_quorum_count,
            )
        );
    }

    #[test]
    fn vec_progress_display_with() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.update_progress(&1, 5);
        progress.update_progress(&2, 3);

        let display = progress.display_with(|f, item| write!(f, "{}={}", item.0, item.1));

        let output = format!("{}", display);
        assert_eq!("{1=5, 2=3, 0=0, 3=0}", output);
    }

    #[test]
    fn vec_progress_increase_to() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [6], |id| (id, 0));

        // Increase from 0 to 5
        progress.increase_to(&1, 5);
        assert_eq!(Some(&(1, 5)), progress.get(&1));

        // Try to decrease from 5 to 3 - should not change
        progress.increase_to(&1, 3);
        assert_eq!(Some(&(1, 5)), progress.get(&1));

        // Increase from 5 to 7
        progress.increase_to(&1, 7);
        assert_eq!(Some(&(1, 7)), progress.get(&1));

        // Try with nonexistent id
        let result = progress.increase_to(&9, 10);
        assert!(result.is_none());
    }

    #[test]
    fn vec_progress_collect_mapped() {
        let quorum_set = vec![btreeset! {0, 1, 2}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [3], |id| (id, 0));

        progress.update_progress(&1, 5);
        progress.update_progress(&2, 3);

        // Collect ids as Vec - order matters after updates (sorted by value descending)
        let ids: Vec<u64> = progress.collect_mapped(|item| item.0);
        assert_eq!(vec![1, 2, 0, 3], ids);

        // Collect values as Vec - order matters after updates (sorted descending)
        let values: Vec<u64> = progress.collect_mapped(|item| item.1);
        assert_eq!(vec![5, 3, 0, 0], values);

        // Collect as Vec of tuples - order matters after updates
        let pairs: Vec<(u64, u64)> = progress.collect_mapped(|item| (item.0, item.1));
        assert_eq!(vec![(1, 5), (2, 3), (0, 0), (3, 0)], pairs);
    }

    #[test]
    fn vec_progress_reset_entry_with() {
        // 7 voters, majority = 4.
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4, 5, 6}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [7], |id| (id, 0));

        progress.update_progress(&0, 12);
        progress.update_progress(&1, 11);
        progress.update_progress(&2, 10);
        progress.update_progress(&3, 9);
        assert_eq!(&9, progress.quorum_accepted());

        // Node 1 log-reverts: its progress falls back to the default value and the
        // entry is moved down, while the quorum-accepted value must be kept.
        let entry = progress.reset_entry_with(&1, |entry| entry.1 = 0);
        assert_eq!(Some(&(1, 0)), entry);
        assert_eq!(
            &9,
            progress.quorum_accepted(),
            "reset never lowers quorum-accepted"
        );
        assert_eq!(
            vec![
                (0, 12),
                (2, 10),
                (3, 9),
                (1, 0),
                (4, 0),
                (5, 0),
                (6, 0),
                (7, 0)
            ],
            progress.entries
        );
        assert_voter_prefix_is_sorted(&progress, "after reset");

        // Node 4 catches up to exactly 10: without the move-down, the reverted node 1
        // would be counted spuriously and 10 would be accepted with only 3 real grants.
        assert_eq!(Some(9), copy_option(progress.update_progress(&4, 10)));
        assert_matches_model(&progress, "after catching up to 10");

        // A real quorum at 10: {0, 2, 4, 5}.
        assert_eq!(Some(10), copy_option(progress.update_progress(&5, 10)));
        assert_matches_model(&progress, "after a real quorum at 10");

        // Resetting a learner or a nonexistent id does not reorder anything.
        assert_eq!(
            Some(&(7, 0)),
            progress.reset_entry_with(&7, |entry| entry.1 = 0)
        );
        assert_eq!(None, progress.reset_entry_with(&9, |entry| entry.1 = 0));
    }

    #[test]
    fn vec_progress_matches_reference_model_with_resets() {
        let cases = [
            (vec![btreeset! {0, 1, 2, 3, 4, 5, 6}], vec![7]),
            (vec![btreeset! {0, 1, 2}, btreeset! {2, 3, 4}], vec![5, 6]),
        ];

        for (case_id, (quorum_set, learners)) in cases.into_iter().enumerate() {
            for seed in 0..32 {
                let mut seed = seed + 3;
                let mut progress =
                    VecProgress::<(u64, u64), _>::new(quorum_set.clone(), learners.clone(), |id| {
                        (id, 0)
                    });

                // The quorum-accepted value never moves backward, so the reference
                // is the running max of the instantaneous model value.
                let mut want = 0;

                for step in 0..128 {
                    // Use high bits: the low bits of this LCG have a short period,
                    // which makes power-of-two moduli cycle in lock-step.
                    let id = (next_random(&mut seed) >> 32) % 8;
                    let context = format!("case-{case_id} seed-{seed} step-{step} id-{id}");

                    if (next_random(&mut seed) >> 32).is_multiple_of(8) {
                        progress.reset_entry_with(&id, |entry| entry.1 = 0);
                    } else {
                        let value = progress.get(&id).map(|entry| entry.1).unwrap_or_default()
                            + next_random(&mut seed) % 7
                            + 1;
                        progress.update_progress(&id, value);
                    }

                    want = want.max(model_quorum_accepted(
                        &progress.quorum_set,
                        &progress.entries,
                    ));
                    assert_eq!(
                        &want,
                        progress.quorum_accepted(),
                        "{context}: entries: {:?}",
                        progress.entries
                    );
                    assert_voter_prefix_is_sorted(&progress, &context);
                }
            }
        }
    }

    #[test]
    fn vec_progress_sub_quorum_commit_regression() {
        let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}];
        let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

        progress.update_progress(&0, 5); // qa = 0
        progress.update_progress(&1, 3); // qa = 0
        progress.update_progress(&2, 4); // qa = 3 ; above-qa region = [0:5, 2:4] (descending, ok)
        progress.update_progress(&2, 10); // voter 2 was already > qa(3) and advances further:
        //   move_up must still run; if it were skipped, the region would become
        //   [0:5, 2:10] (NOT descending) and the next update would falsely accept 6.
        let qa = progress.update_progress(&3, 6);

        // True match values: {0:5, 1:3, 2:10, 3:6, 4:0}; sorted desc = 10,6,5,3,0 -> 3rd-largest =
        // 5. Only voters {2,3} reached 6 -> 2 voters -> NOT a majority.
        // Expected quorum_accepted = 5.
        assert_eq!(Some(&5), qa);
    }
}
