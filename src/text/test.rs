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

fn main() {
    let params = editor::cli::get_params();
    let path = params.paths.first().unwrap();
    log::info!("Start: {}", path);
    let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap();
    run(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_text() -> Rope {
        Rope::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###)
    }
}



