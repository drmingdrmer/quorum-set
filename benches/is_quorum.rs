use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use maplit::btreeset;
use quorum_set::Node;
use quorum_set::QuorumSet;
use quorum_set::QuorumTree;

fn bench_quorum_set_tree_ids_slice(c: &mut Criterion) {
    let m12345 = QuorumTree::new(3, [1, 2, 3, 4, 5].map(Node::Id)).unwrap();
    let x = [1, 2, 3, 6, 7];
    c.bench_function("quorum_set_tree_ids_slice", |b| {
        b.iter(|| m12345.is_quorum(black_box(x.iter())))
    });
}

fn bench_quorum_set_btreeset_ids_slice(c: &mut Criterion) {
    let m12345678 = btreeset! {1,2,3,4,5,6,7,8};
    let x = [1, 2, 3, 6, 7];
    c.bench_function("quorum_set_btreeset_ids_slice", |b| {
        b.iter(|| m12345678.is_quorum(black_box(x.iter())))
    });
}

fn bench_quorum_set_vec_of_btreeset_ids_slice(c: &mut Criterion) {
    let m12345_678 = vec![btreeset! {1,2,3,4,5}, btreeset! {6,7,8}];
    let x = [1, 2, 3, 6, 7];
    c.bench_function("quorum_set_vec_of_btreeset_ids_slice", |b| {
        b.iter(|| m12345_678.is_quorum(black_box(x.iter())))
    });
}

fn bench_quorum_set_vec_of_btreeset_ids_btreeset(c: &mut Criterion) {
    let m12345_678 = vec![btreeset! {1,2,3,4,5}, btreeset! {6,7,8}];
    let x = btreeset! {1,2,3,6,7};
    c.bench_function("quorum_set_vec_of_btreeset_ids_btreeset", |b| {
        b.iter(|| m12345_678.is_quorum(black_box(x.iter())))
    });
}

criterion_group!(
    benches,
    bench_quorum_set_tree_ids_slice,
    bench_quorum_set_btreeset_ids_slice,
    bench_quorum_set_vec_of_btreeset_ids_slice,
    bench_quorum_set_vec_of_btreeset_ids_btreeset,
);
criterion_main!(benches);
