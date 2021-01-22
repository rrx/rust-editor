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
    wrap.update_lines(&text, &port);
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
            let mut wrap = LineWrap::default();
            wrap.update_spec(sx, sy);
            Self {
                wrap
            }
        }
        fn update(&mut self, text: &Rope, port: &ViewPort) {
            self.wrap.update_lines(&text, &port);
        }
        fn update_string(&mut self, s: &str, port: &ViewPort) {
            self.update(&Rope::from_str(s), port);
        }
    }

    use ViewChar::*;
    #[test]
    fn test_linewrap_1() {
        let mut app = TestApp::new(10,10);
        let port = ViewPort::default();
        app.update_string("aa\tb\tc\td", &port);
        println!("x: {:#?}", app);
        assert_eq!(app.wrap.get(0,1).e, Tab);
        assert_eq!(app.wrap.get(1,1).e, Char('c'));
        assert_eq!(app.wrap.get(2,1).e, NOP);
    }

    #[test]
    fn test_linewrap_2() {
        let mut text = Rope::from_str("1234");
        text.insert_char(1, 'a');
        let mut text2 = text.clone();
        text2.insert_char(2,'b');
        println!("{:?}", (text, text2));
    }

}



