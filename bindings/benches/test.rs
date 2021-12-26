use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_update(c: &mut Criterion) {
}

criterion_group!(benches, criterion_update);
criterion_main!(benches);
