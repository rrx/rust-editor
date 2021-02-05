use criterion::{black_box, criterion_group, criterion_main, Criterion};

use editor::text::*;
use ropey::Rope;
use std::fs::File;
use std::io;

fn get_text() -> Rope {
    Rope::from_str(
        r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###,
    )
}

fn criterion_update(c: &mut Criterion) {
    let fb = FileBuffer::from_string(&r###"test
    line2
    estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
    asdf
    "###.to_string());

    let mut bb = BufferBlock::new(fb);
    bb.resize(10, 10, 0, 0, 0);
    //let (sx, sy) = (10,10);
    //wrap.update_spec(sx, sy);
    c.bench_function("update", |b| {
        b.iter(|| {
            bb.clear().update();
            //bb.update();
            //wrap.update_lines(&text, &port);
        })
    });
}

criterion_group!(benches, criterion_update);
criterion_main!(benches);
