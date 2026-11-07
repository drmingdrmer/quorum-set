use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::btree_set;

use crate::quorum::HierarchicalQuorumSetError;
use crate::quorum::QuorumIntersection;
use crate::quorum::QuorumSet;

/// One group of a [`HierarchicalQuorumSet`]: its members, which are IDs and child groups, and
/// how many of them a set must reach to reach the group.
///
/// A set reaches an ID by holding it, and a child group by reaching enough members of that group
/// in turn. So the groups form a tree whose leaves are IDs, and every level applies the same
/// test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group<ID> {
    /// The number of members of the group that an accepted set must reach to reach the group.
    pub quorum_size: u64,

    /// The IDs that the group holds directly.
    pub ids: BTreeSet<ID>,

    /// The child groups, by name.
    pub groups: BTreeMap<String, Group<ID>>,
}

impl<ID> Group<ID>
where ID: Ord + Clone
{
    /// Returns a group of `ids`, which a set reaches by holding `quorum_size` of them.
    pub fn of_ids<I>(quorum_size: u64, ids: I) -> Self
    where I: IntoIterator<Item = ID> {
        Self {
            quorum_size,
            ids: ids.into_iter().collect(),
            groups: BTreeMap::new(),
        }
    }

    /// Returns a group of the child groups `groups`, each given with its name, which a set reaches
    /// by reaching `quorum_size` of them. A later group replaces an earlier one with the same name.
    pub fn of_groups<I, K>(quorum_size: u64, groups: I) -> Self
    where
        I: IntoIterator<Item = (K, Group<ID>)>,
        K: Into<String>,
    {
        let mut by_name = BTreeMap::new();
        for (name, group) in groups {
            by_name.insert(name.into(), group);
        }
        Self {
            quorum_size,
            ids: BTreeSet::new(),
            groups: by_name,
        }
    }

    /// Returns the number of members: the IDs and the child groups.
    fn member_count(&self) -> u64 {
        let member_count = self.ids.len() + self.groups.len();
        member_count as u64
    }

    /// Returns whether `candidates` reach this group.
    fn is_reached_by(&self, candidates: &BTreeSet<&ID>) -> bool {
        let held_ids = self.ids.iter().filter(|id| candidates.contains(id));
        let held = held_ids.count() as u64;
        let reached_groups = self.groups.values().filter(|group| group.is_reached_by(candidates));
        let reached = reached_groups.count() as u64;
        held + reached >= self.quorum_size
    }

    /// Adds the IDs of this group and of every group below it to `ids`.
    fn collect_ids(&self, ids: &mut BTreeSet<ID>) {
        ids.extend(self.ids.iter().cloned());
        for group in self.groups.values() {
            group.collect_ids(ids);
        }
    }

    /// Returns the group with the same members whose quorum size, here and in every group below,
    /// is `member_count - quorum_size + 1`.
    fn dual(&self) -> Self {
        let quorum_size = self.member_count() - self.quorum_size + 1;
        let mut groups = BTreeMap::new();
        for (name, group) in self.groups.iter() {
            groups.insert(name.clone(), group.dual());
        }
        Self {
            quorum_size,
            ids: self.ids.clone(),
            groups,
        }
    }

    /// Returns whether `self` and `other` hold the same IDs and the same child group names, here
    /// and in every group below, with two quorum sizes that sum to more than the member count of
    /// each group.
    fn proves_intersection(&self, other: &Self) -> bool {
        let same_ids = self.ids == other.ids;
        let same_names = self.groups.keys().eq(other.groups.keys());
        if !same_ids || !same_names {
            return false;
        }

        let quorum_sizes = self.quorum_size + other.quorum_size;
        if quorum_sizes <= self.member_count() {
            return false;
        }

        let children = self.groups.values().zip(other.groups.values());
        for (group, other_group) in children {
            let proved = group.proves_intersection(other_group);
            if !proved {
                return false;
            }
        }
        true
    }
}

/// A quorum rule over a tree of groups: it accepts an ID set that reaches the root group.
///
/// A [`Group`] holds IDs, child groups, or both: its members. A set reaches a group when the
/// IDs of the group that it holds, plus the child groups that it reaches, number at least the
/// quorum size of the group.
///
/// Each group has a name that is stable among its siblings, such as the name of a zone, and each
/// ID belongs to exactly one group. [`new()`](Self::new) checks that every group of the tree
/// holds a member and has `1 <= quorum_size <= member_count`, and that no ID appears twice in
/// the tree. So every group can be reached, and the set of all IDs is a quorum.
///
/// The intersection check proves `Some(true)` for two rules over the same tree when their quorum
/// sizes sum to more than the member count of every group, as a rule and its
/// [`dual()`](Self::dual) do. It answers `None` for every other pair.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use std::collections::BTreeSet;
///
/// use quorum_set::{Group, HierarchicalQuorumSet, QuorumIntersection, QuorumSet};
///
/// // Zone a has racks r1 = {1, 2} and r2 = {3, 4}, zone b holds IDs 5 and 6, and ID 7
/// // sits at the root. A quorum reaches 2 of zone a, zone b and ID 7. It reaches zone a
/// // through an ID in each rack, and zone b through both of its IDs.
/// let a = Group::of_groups(2, [
///     ("r1", Group::of_ids(1, [1, 2])),
///     ("r2", Group::of_ids(1, [3, 4])),
/// ]);
/// let b = Group::of_ids(2, [5, 6]);
/// let root = Group {
///     quorum_size: 2,
///     ids: BTreeSet::from([7]),
///     groups: BTreeMap::from([("a".to_string(), a), ("b".to_string(), b)]),
/// };
/// let rule = HierarchicalQuorumSet::new(root).unwrap();
///
/// let accepts = rule.is_quorum([1, 3, 7].iter());
/// assert!(accepts);
/// // Rack r2 is lost, so only zone b is reached.
/// let accepts = rule.is_quorum([1, 2, 5, 6].iter());
/// assert!(!accepts);
///
/// // The quorums {1, 3, 7} and {2, 4, 5, 6} both reach zone a, through other IDs of each
/// // rack. The check proves nothing, because the two quorum sizes of a rack sum to only its
/// // member count.
/// let intersects = rule.intersects_with(&rule);
/// assert_eq!(None, intersects);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchicalQuorumSet<ID> {
    /// The root group, which holds every ID of the rule in its tree.
    root: Group<ID>,
}

impl<ID> HierarchicalQuorumSet<ID>
where ID: Ord + Clone
{
    /// Builds a rule that accepts an ID set reaching `root`.
    ///
    /// Returns [`HierarchicalQuorumSetError`] for the first problem in a walk of the tree that
    /// visits a group before the groups below it, and sibling groups in name order: a group that
    /// holds no member, a group without `1 <= quorum_size <= member_count`, or an ID that a
    /// visited group already holds.
    pub fn new(root: Group<ID>) -> Result<Self, HierarchicalQuorumSetError<ID>> {
        let mut group_of = BTreeMap::new();
        check_group(&root, &[], &mut group_of)?;

        let rule = Self { root };
        Ok(rule)
    }

    /// Returns the root group, which holds every ID of the rule in its tree.
    pub fn root(&self) -> &Group<ID> {
        &self.root
    }

    /// Returns the rule over the same tree that accepts exactly the sets that meet every quorum of
    /// `self`.
    ///
    /// A quorum of `self` that misses a set `S` takes its IDs from outside `S`. Because no ID
    /// appears twice in the tree, it can reach the members of a group independently: each ID
    /// outside `S`, and each child group that `S` does not block. `S` blocks a group when it leaves
    /// fewer than `quorum_size` members reachable, so when it holds or blocks
    /// `member_count - quorum_size + 1` members: the IDs that it holds and the child groups that
    /// it blocks. `S` meets every quorum exactly when it blocks the root group. So the result keeps
    /// the tree and gives every group that number as its quorum size.
    pub fn dual(&self) -> Self {
        let root = self.root.dual();
        Self { root }
    }
}

impl<ID> QuorumSet for HierarchicalQuorumSet<ID>
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
        self.root.is_reached_by(&candidates)
    }

    fn ids(&self) -> Self::Iter {
        let mut ids = BTreeSet::new();
        self.root.collect_ids(&mut ids);
        ids.into_iter()
    }
}

/// Sufficient quorum intersection for two rules over the same tree.
///
/// A quorum of each rule reaches at least `quorum_size` members of the root group. When the two
/// quorum sizes sum to more than the member count, two quorums reach a common member. A common
/// ID is shared by both quorums, and a common child group repeats the argument one level down. So
/// two quorums share an ID when the two quorum sizes of every group sum to more than its member
/// count. The [`dual()`](HierarchicalQuorumSet::dual) of a rule gives every group the quorum size
/// that makes this sum exceed its member count by exactly 1, so the check proves every rule against
/// its dual. The check runs in O(n).
impl<ID> QuorumIntersection<HierarchicalQuorumSet<ID>> for HierarchicalQuorumSet<ID>
where ID: Ord + Clone
{
    /// Return `Some(true)` when both rules hold the same IDs and the same child group names in
    /// every group, and the two quorum sizes of every group sum to more than its member count.
    ///
    /// Return `None` otherwise, including for rules over different trees, such as after an ID
    /// moves to another group: the check proves nothing for them. This check never returns
    /// `Some(false)`.
    fn intersects_with(&self, other: &HierarchicalQuorumSet<ID>) -> Option<bool> {
        let proved = self.root.proves_intersection(&other.root);
        if proved { Some(true) } else { None }
    }
}

/// Returns an error unless `group`, at `path`, and every group below it hold a member and have
/// `1 <= quorum_size <= member_count`, and no ID appears twice. `group_of` maps each ID of
/// the groups visited before to the path of its group.
fn check_group<ID>(
    group: &Group<ID>,
    path: &[String],
    group_of: &mut BTreeMap<ID, Vec<String>>,
) -> Result<(), HierarchicalQuorumSetError<ID>>
where
    ID: Ord + Clone,
{
    let member_count = group.member_count();
    if member_count == 0 {
        let empty = HierarchicalQuorumSetError::EmptyGroup {
            group: path.to_vec(),
        };
        return Err(empty);
    }

    let size_fits = (1..=member_count).contains(&group.quorum_size);
    if !size_fits {
        let invalid = HierarchicalQuorumSetError::InvalidQuorumSize {
            group: path.to_vec(),
            quorum_size: group.quorum_size,
            member_count,
        };
        return Err(invalid);
    }

    for id in &group.ids {
        let Some(first) = group_of.insert(id.clone(), path.to_vec()) else {
            continue;
        };
        let shared = HierarchicalQuorumSetError::SharedId {
            id: id.clone(),
            first,
            second: path.to_vec(),
        };
        return Err(shared);
    }

    for (name, child) in group.groups.iter() {
        let mut child_path = path.to_vec();
        child_path.push(name.clone());
        check_group(child, &child_path, group_of)?;
    }
    Ok(())
}
