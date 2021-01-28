use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};

mod scroll;
mod render;
mod wrap;
pub mod cursor;
mod bufferview;
mod viewspec;
mod app;
pub mod smart;
pub mod linewrap;
pub mod viewport;
mod viewrow;
pub mod rowiter;
pub mod bufferlist;
pub mod textbuffer;
pub mod buffer;
pub mod lineworker;
pub mod display;
pub mod search;
pub mod window;

pub use smart::*;
pub use window::*;
pub use search::*;
pub use display::*;
pub use bufferview::*;
pub use viewspec::*;
pub use viewrow::*;
pub use linewrap::*;
pub use viewport::*;
pub use rowiter::*;
pub use bufferlist::*;
pub use textbuffer::*;
pub use buffer::*;
pub use app::*;
pub use wrap::WrapValue;
pub use cursor::*;
pub use lineworker::*;
pub use crate::bindings::parser::Motion;

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Easy
}
impl Default for Mode {
    fn default() -> Self { Self::Normal }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Command {
    Insert(char),
    Backspace,
    Motion(usize, Motion),
    Search(String),
    RemoveChar(i32),
    Mode(Mode),
    Quit,
    Save,
    Mouse(u16, u16),
    Scroll(i16),
    ScrollPage(i8),
    Line(i64),
    LineNav(i32),
    Resize(u16,u16),
    MoveCursorY(i32),
    MoveCursorX(i32),
    BufferNext,
    BufferPrev,
    Test,
    Refresh
}

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

    fn dump(&mut self) {
        let commands = self.render_view();
        for command in &commands {
            println!("{:?}", command);
        }
        println!("{:#?}", self);
        println!("Commands: {}", commands.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
