use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;

fn main() {
    let params = editor::cli::get_params();
    let path = params.paths.first().unwrap();
    log::info!("Start: {}", path);
    let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap();
    let port = ViewPort::default();
    let mut wrap = LineWrap::default();
    let (sx, sy) = (10,10);
    wrap.update_spec(sx, sy);
    wrap.update_port(port);
    wrap.update_lines(&text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestApp<'a> {
        wrap: LineWrap<'a>,
    }

    impl<'a> TestApp<'a> {
        fn new(sx: u16, sy: u16) -> Self {
            let port = ViewPort::default();
            let mut wrap = LineWrap::default();
            wrap.update_spec(sx, sy);
            wrap.update_port(port);
            Self {
                wrap
            }
        }
        fn update(&mut self, text: &Rope) {
            self.wrap.update_lines(&text);
        }
        fn update_string(&mut self, s: &str) {
            self.update(&Rope::from_str(s));
        }
    }

    fn get_text() -> Rope {
        Rope::from_str(r###"extern crate ropey;

use std::fs::File;
                                
use std::io;

use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};

struct TextBuffer {
    text: Rope,
    path: String,
    dirty: bool,
}
asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf asdf
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###)
    }

    use ViewChar::*;
    #[test]
    fn test_linewrap_1() {
        let mut app = TestApp::new(10,10);
        let text = get_text();
        //app.update(&text);
        app.update_string("aa\tb\tc\td");
        println!("x: {:#?}", app);
        assert_eq!(app.wrap.get(0,1).e, Tab);
        assert_eq!(app.wrap.get(1,1).e, Char('c'));
        assert_eq!(app.wrap.get(2,1).e, NOP);
    }

}



