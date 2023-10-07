use criterion::{criterion_group, criterion_main, Criterion};

use editor_core::Buffer;
use editor_core::{BufferConfig, Command, Motion, ViewPos};
use editor_tui::BufferBlock;

fn criterion_update(c: &mut Criterion) {
    let fb = Buffer::from_string(&r###"test
    line2
    estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
    asdf
    "###.to_string());

    let view_pos = ViewPos {
        w: 10,
        h: 10,
        x0: 0,
        y0: 0,
    };
    let mut bb = BufferBlock::new(fb, view_pos.clone());
    bb.resize(view_pos, 0);
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
