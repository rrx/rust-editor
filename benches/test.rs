use criterion::{black_box, criterion_group, criterion_main, Criterion};

use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;

fn run(text: &Rope) {
    let port = ViewPort::default();
    let mut wrap = LineWrap::default();
    wrap.update_spec(10,10);
    wrap.update_port(port);
    wrap.update_lines(&text);
    log::info!("x: {:?}", wrap);
    log::info!("x: {:?}", (
            wrap.get(0,0),
            wrap.get(9,0),
            wrap.get(0,6),
            wrap.get(1,6),
            wrap.get(5,6),
            wrap.get(1,10),
    ));

}

fn get_text() -> Rope {
    Rope::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###)
}

fn criterion_update(c: &mut Criterion) {
    let text = get_text();
    c.bench_function("update", |b| b.iter(|| run(&text)));
}


criterion_group!(benches, criterion_update);
criterion_main!(benches);



