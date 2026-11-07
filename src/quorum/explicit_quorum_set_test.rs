use std::collections::BTreeSet;

use maplit::btreeset;

use crate::quorum::ExplicitQuorumSet;
use crate::quorum::QuorumIntersection;
use crate::quorum::QuorumSet;
use crate::quorum::testing::accepted;
use crate::quorum::testing::assert_quorum_set_contract;
use crate::quorum::testing::holding_one_of;
use crate::quorum::threshold_quorum_set_test::all_subsets;
use crate::quorum::verify_intersection;

/// Returns the rule that lists `quorums`, none of which is empty.
fn explicit(quorums: Vec<BTreeSet<u64>>) -> ExplicitQuorumSet<u64> {
    ExplicitQuorumSet::new(quorums).expect("no listed quorum is empty")
}

#[test]
fn test_new_keeps_sorted_minimal_quorums() {
    let quorums = vec![
        btreeset! {2, 3},
        btreeset! {1, 2, 4},
        btreeset! {1, 4},
        btreeset! {2, 3},
        btreeset! {3, 4, 5},
    ];
    let rule = explicit(quorums);

    // {1,2,4} holds {1,4}, and the second {2,3} repeats the first.
    let want = [btreeset! {1, 4}, btreeset! {2, 3}, btreeset! {3, 4, 5}];
    assert_eq!(&want, rule.quorums());

    let ids: Vec<u64> = rule.ids().collect();
    assert_eq!(vec![1, 2, 3, 4, 5], ids);
}

#[test]
fn test_new_drops_ids_of_redundant_quorums() {
    // ID 3 is only in {1,2,3}, which {1,2} makes redundant, so ID 3 never changes a result.
    let rule = explicit(vec![btreeset! {1, 2, 3}, btreeset! {1, 2}]);

    assert_eq!(&[btreeset! {1, 2}], rule.quorums());
    let ids: Vec<u64> = rule.ids().collect();
    assert_eq!(vec![1, 2], ids);
}

#[test]
fn test_new_rejects_empty_quorum() {
    let rule = ExplicitQuorumSet::<u64>::new([btreeset! {1}, BTreeSet::new()]);
    assert_eq!(None, rule);
}

#[test]
fn test_rule_without_quorum_accepts_no_set() {
    let rule = explicit(vec![]);

    let accepts = rule.is_quorum([1, 2].iter());
    assert!(!accepts);
    let ids: Vec<u64> = rule.ids().collect();
    assert_eq!(Vec::<u64>::new(), ids);
}

#[test]
fn test_accepts_sets_that_hold_a_listed_quorum() {
    let quorums = vec![btreeset! {1, 2}, btreeset! {2, 3}, btreeset! {3, 4}];
    let rule = explicit(quorums.clone());

    for subset in all_subsets(&btreeset! {1, 2, 3, 4}) {
        let want = quorums.iter().any(|quorum| quorum.is_subset(&subset));
        let got = rule.is_quorum(subset.iter());
        assert_eq!(want, got, "{subset:?}");
    }
}

#[test]
fn test_small_lists_accept_exactly_the_sets_holding_a_listed_quorum() {
    let subsets = all_subsets(&btreeset! {1, 2, 3});
    let mut quorums = subsets.clone();
    quorums.retain(|quorum| !quorum.is_empty());

    // Every list of at most two quorums over 3 IDs, including repeated and redundant quorums.
    let mut lists = vec![vec![]];
    for a in &quorums {
        lists.push(vec![a.clone()]);
        for b in &quorums {
            lists.push(vec![a.clone(), b.clone()]);
        }
    }

    for list in lists {
        let rule = explicit(list.clone());
        assert_quorum_set_contract(&rule);
        let got = accepted(&rule, &subsets);
        let want = holding_one_of(&list, &subsets);
        assert_eq!(want, got, "{list:?}");
    }
}

#[test]
fn test_is_quorum_ignores_unknown_and_repeated_ids() {
    let rule = explicit(vec![btreeset! {1, 2}]);

    let accepts = rule.is_quorum([2, 9, 2, 1].iter());
    assert!(accepts);
    let accepts = rule.is_quorum([1, 1, 9].iter());
    assert!(!accepts);
    let accepts = rule.is_quorum([].iter());
    assert!(!accepts);
}

#[test]
fn test_intersection_is_exact() {
    let ids = btreeset! {1, 2, 3};
    let mut quorums = all_subsets(&ids);
    quorums.retain(|quorum| !quorum.is_empty());

    // Every rule that lists at most two quorums over 3 IDs.
    let mut rules = vec![explicit(vec![])];
    for a in &quorums {
        rules.push(explicit(vec![a.clone()]));
        for b in &quorums {
            rules.push(explicit(vec![a.clone(), b.clone()]));
        }
    }

    for a in &rules {
        for b in &rules {
            let want = verify_intersection(a, b);
            let got = a.intersects_with(b);
            assert_eq!(Some(want), got, "a={a:?}, b={b:?}");
        }
    }
}

#[test]
fn test_disjoint_quorums() {
    let a = explicit(vec![btreeset! {1, 2}, btreeset! {3, 4}]);
    let b = explicit(vec![btreeset! {1, 3}, btreeset! {5}]);

    // {1,2} meets {1,3}, then misses {5}.
    let got = a.disjoint_quorums(&b);
    assert_eq!(Some((&btreeset! {1, 2}, &btreeset! {5})), got);

    // Every listed quorum of `a` meets {1,3,4} and {2,3}.
    let c = explicit(vec![btreeset! {1, 3, 4}, btreeset! {2, 3}]);
    let got = a.disjoint_quorums(&c);
    assert_eq!(None, got);
}
