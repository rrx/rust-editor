use criterion::{black_box, criterion_group, criterion_main, Criterion};

use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;

fn get_text() -> Rope {
    Rope::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###)
}

fn criterion_update(c: &mut Criterion) {
    let text = get_text();
    let port = ViewPort::default();
    let mut wrap = LineWrap::default();
    let (sx, sy) = (10,10);
    wrap.update_spec(sx, sy);
    c.bench_function("update", |b| b.iter(|| {
        wrap.update_lines(&text, &port);
    }));
}


criterion_group!(benches, criterion_update);
criterion_main!(benches);



