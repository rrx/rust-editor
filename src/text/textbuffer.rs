use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use super::*;
use std::fs::File;
use std::io;

#[derive(Debug)]
pub struct TextBuffer {
    pub text: Rope,
    pub path: String,
    pub dirty: bool,
    pub line_offset: usize,
    pub line_current: usize,
    pub mode: Mode,
    // viewport start/cursor/end
    pub char_start: usize,
    pub char_current: usize,
    pub char_end: usize,

    pub view: EditorView
}

#[derive(Debug)]
pub struct EditorView {
    pub size: (u16, u16),
    pub cursor: (u16, u16),
    pub cursor_x_hint: u16,
    pub vsy: u16,
    pub vsx: u16,
    pub r_info: u16,
    pub r_command: u16,
    pub debug: String,
    pub wraps: Vec<wrap::WrapValue>,
    pub wrap_current: u16
}

impl EditorView {
    fn new() -> Self {
        Self {
            size: (0,0),
            cursor: (0,0),
            cursor_x_hint: 0,
            vsy: 0,
            vsx: 0,
            r_info: 0,
            r_command: 0,
            debug: String::new(),
            wraps: Vec::new(),
            wrap_current: 0
        }
    }

}
impl TextBuffer {
    pub fn new(text: Rope, path: &str) -> Self {
        Self {
            text: text,
            path: path.to_string(),
            dirty: false,
            mode: Mode::default(),
            line_offset: 0,
            line_current: 0,
            char_start: 0,
            char_current: 0,
            char_end: 0,
            view: EditorView::new()
        }
    }

    pub fn from_path(path: &str) -> io::Result<TextBuffer> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(TextBuffer::new(text, path))
    }

    pub fn from_str(s: &str) -> Self {
        let text = Rope::from_str(s);
        Self::new(text, "")
    }

    pub fn set_size(&mut self, x: u16, y: u16) {
        self.view.size = (x, y);
        // viewport size, create gutter and footer
        self.view.vsy = y - 2;
        self.view.vsx = x - 5;
        self.view.r_info = y - 2;
        self.view.r_command = y - 1;
        self.update_window(self.char_start);
    }

    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.view.cursor = (x, y);
    }

    pub fn pos(&self) -> (u16, u16) {
        return self.view.cursor;
    }

    pub fn cursor(&self) -> (u16, u16) {
        return self.view.cursor;
    }

    pub fn size(&self) -> (u16, u16) {
        return self.view.size;
    }

    fn get_line<'a>(&'a self, idx: usize) -> RopeSlice<'a> {
        self.text.line(idx)
    }

    fn bytes<'a>(&'a self) -> Bytes<'a> {
        self.text.bytes()
    }

    fn chars<'a>(&'a self) -> Chars<'a> {
        self.text.chars()
    }

    fn lines<'a>(&'a self) -> Lines<'a> {
        self.text.lines()
    }

    fn lines_at<'a>(&'a self, line_inx: usize) -> Lines<'a> {
        self.text.lines_at(line_inx)
    }

    fn chunks<'a>(&'a self) -> Chunks<'a> {
        self.text.chunks()
    }

    fn edit(&mut self, start: usize, end: usize, text: &str) {
        if start != end {
            self.text.remove(start..end);
        }
        if !text.is_empty() {
            self.text.insert(start, text);
        }
        self.dirty = true;
    }

    pub fn command(&mut self, evt: Command) {
        match evt {
            Command::Mode(m) => {
                self.mode = m;
            }
            Command::Insert(c) => {
                self.insert(c);
            }
            Command::MoveCursorX(dx) => {
                self.move_cursor_x(self.char_current, dx);
            }
            Command::MoveCursorY(dy) => {
                self.move_cursor_y(self.char_current, dy);
            }
            Command::Quit => (),
            Command::ScrollPage(dy) => {
                let xdy = self.view.vsy as f32 / dy as f32;
                self.scroll(xdy as i32);
            }
            Command::Scroll(dy) => {
                self.scroll(dy as i32);
            }

            Command::LineNav(x) => {
                self.line_move(x);
            }

            // Goto a line
            Command::Line(line) => {
                self.scroll_line(line);
            }

            Command::Resize(a, b) => {
                self.set_size(a, b);
            }

            Command::Mouse(x, y) => {
                if x >= 6 && y < self.view.vsy {
                    let mut cx = x as usize - 6;
                    let cy = y as usize;
                    let w = self.view.wraps[cy];
                    let line_length = w.c1 - w.c0;
                    if cx >= line_length {
                        cx = line_length - 1;
                    }
                    let c = w.c0 + cx;
                    self.update_window(c);
                }
            }
            _ => ()
        }
    }

    pub fn dump(&mut self) {
        let commands = self.render_view();
        for command in &commands {
            println!("{:?}", command);
        }
        println!("{:#?}", self);
        println!("Commands: {}", commands.len());
    }
}


impl TextBuffer {
    pub fn line_move(&mut self, x: i32) {
        let w = self.char_to_wrap(self.char_current).unwrap();
        let mut lc = x;
        let line_length = w.lc1 - w.lc0;
        if x < 0 {
            lc += line_length as i32;
        }
        if lc < 0 || line_length == 0 {
            lc = 0;
        } else if lc >= line_length as i32 {
            lc = line_length as i32 - 1;
        }
        let c = w.lc0 + lc as usize;
        if c != self.char_current {
            self.update_window(c as usize);
        }
    }

    pub fn move_cursor_x(&mut self, c0: usize, dx: i32) {
        self._move_cursor_x(c0, dx, false);
    }

    pub fn _move_cursor_x(&mut self, c0: usize, dx: i32, constrain: bool) {
        let mut c = c0 as i32 + dx;
        if c < 0 {
            c = 0;
        } else if c > self.text.len_chars() as i32 {
            c = self.text.len_chars() as i32;
        }

        let mut c1 = c as usize;
        if constrain {
            // restrict x movement to the specific line
            let w = self.char_to_wrap(c0).unwrap();
            let line_length = w.lc1 - w.lc0;
            if c1 < w.lc0 {
                c1 = w.lc0;
            } else if c1 >= w.lc1 {
                if line_length > 0 {
                    c1 = w.lc1 - 1;
                } else {
                    c1 = w.lc0;
                }
            }
        }

        if c0 != c1 {
            let w = self.char_to_wrap(c1).unwrap();
            let hint = c1 - w.c0;
            self.view.cursor_x_hint = hint as u16;
            self.update_window(c1);
        }
    }

    pub fn move_cursor_y(&mut self, c0: usize, dy: i32) {
        let w = self.delta_wrap(c0, dy);

        // use x hint
        let mut c = w.c0 + self.view.cursor_x_hint as usize;
        if c >= w.lc1 && w.lc0 < w.lc1 {
            c = w.lc1 - 1;
        }
        self.update_window(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_x() {
        let mut buf = TextBuffer::from_str(r###"0123456
0123456
0123456
0123456
0123456 "###);
        buf.set_size(20, 8);
        let mut c = 0;
        assert_eq!(0, buf.view.cursor.0);
        buf.move_cursor_x(c,1);
        buf.dump();
        println!("W: {:?}", (buf.view.cursor));
        assert_eq!(1, buf.view.cursor.0);
    }

    fn get_buf() -> TextBuffer {
        let mut buf = TextBuffer::from_str(r###"test
line2
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###);
        buf.set_size(20, 10);
        buf
    }

    //#[test]
    fn test_move() {
        let mut buf = get_buf();
        buf.set_size(20, 10);
        for command in buf.render_view() {
            println!("{:?}", command);
        }

        // move down
        buf.command(Command::MoveCursorY(1));
        assert_eq!(buf.pos().1, 1);

        //move back up
        buf.command(Command::MoveCursorY(-1));
        assert_eq!(buf.pos().1, 0);

        // try to move out of bounds
        buf.command(Command::MoveCursorY(-1));
        assert_eq!(buf.pos().1, 0);

        // try to go out of bounds at the end of the file
        buf.command(Command::MoveCursorY(100));
        assert_eq!(buf.pos().1, 10);
    }

    //#[test]
    fn test_line() {
        let mut buf = get_buf();
        buf.set_size(20, 10);

        buf.command(Command::Line(-1));
        assert_eq!(buf.cursor().1, 3);
    }

}

