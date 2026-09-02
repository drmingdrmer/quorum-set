use std::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use maplit::btreeset;
use quorum_set::VecProgress;

fn bench_progress_update_progress_01234_567(c: &mut Criterion) {
    let quorum_set = vec![btreeset! {0, 1, 2, 3, 4}, btreeset! {5, 6, 7}];
    let mut progress = VecProgress::<(u64, u64), _>::new(quorum_set, [], |id| (id, 0));

    let mut id = 0u64;
    let mut values = [0, 1, 2, 3, 4, 5, 6, 7];
    c.bench_function("progress_update_progress_01234_567", |b| {
        b.iter(|| {
            id = (id + 1) & 7;
            values[id as usize] += 1;
            let v = values[id as usize];

            progress.update_progress(&black_box(id), black_box(v));
        })
    });
}

criterion_group!(benches, bench_progress_update_progress_01234_567);
criterion_main!(benches);
