use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use crate::frontend::{DrawCommand, ReadEvent};

mod scroll;
mod render;
mod wrap;
mod cursor;

#[derive(Debug)]
pub enum EditMode {
    Insert,
    Normal,
    Command
}

#[derive(Debug)]
pub struct TextBuffer {
    pub text: Rope,
    pub path: String,
    pub dirty: bool,
    pub line_offset: usize,
    pub line_current: usize,

    // viewport start/end
    pub char_start: usize,
    pub char_end: usize,

    pub mode: EditMode,
    pub view: EditorView
}

#[derive(Debug)]
pub struct EditorView {
    pub size: (u16, u16),
    pub cursor: (u16, u16),
    pub vsy: u16,
    pub vsx: u16,
    pub rInfo: u16,
    pub rCmd: u16,
    pub debug: String,
    pub wraps: Vec<wrap::WrapValue>
}

impl EditorView {
    fn new() -> Self {
        Self {
            size: (0,0),
            cursor: (0,0),
            vsy: 0,
            vsx: 0,
            rInfo: 0,
            rCmd: 0,
            debug: String::new(),
            wraps: Vec::new()
        }
    }

}
impl TextBuffer {
    pub fn new(text: Rope, path: &str) -> Self {
        Self {
            text: text,
            path: path.to_string(),
            dirty: false,
            line_offset: 0,
            line_current: 0,
            char_start: 0,
            char_end: 0,
            mode: EditMode::Normal,
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

    pub fn set_mode(&mut self, mode: EditMode) {
        self.mode = mode;
    }

    pub fn set_size(&mut self, x: u16, y: u16) {
        self.view.size = (x, y);
        // viewport size, create gutter and footer
        self.view.vsy = y - 2;
        self.view.vsx = x - 5;
        self.view.rInfo = y - 2;
        self.view.rCmd = y - 1;
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

    fn normalize(&mut self) {
        let line_count = self.text.len_lines() - 1;

        if self.line_offset >= line_count {
            self.line_offset = line_count - 1;
        }

        if self.line_current >= line_count {
            self.line_current = line_count - 1;
        }

        if self.line_current < self.line_offset {
            self.line_offset = self.line_current;
        } else if self.line_current >= self.line_offset + self.view.vsy as usize {
            self.line_offset = self.line_current;
        }

    }

    pub fn command(&mut self, evt: ReadEvent) {
        match evt {
            ReadEvent::MoveCursorX(dx) => {
                self.move_cursor_x(dx);
            }
            ReadEvent::MoveCursorY(dy) => {
                self.move_cursor_y(dy);
            }
            ReadEvent::Stop => (),
            ReadEvent::Scroll(dy) => {
                self.scroll(dy as i32);
            }

            // Goto a line
            ReadEvent::Line(line) => {
                // 0 is the start
                // negative lines is the number of lines from the end of the file
                let lines: usize = self.text.len_lines() - 1;
                let current: usize;
                let mut offset: usize;
                if line < 0 {
                    current = lines - i64::abs(line) as usize;
                } else {
                    current = line as usize;
                }

                let w = self.line_to_wrap(current).unwrap();
                self.char_start = w.c0;
            }

            ReadEvent::Resize(a, b) => {
                self.set_size(a, b);
            }

            ReadEvent::Mouse(x, y) => {
                self.set_cursor(x, y);
            }
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
        buf.set_cursor(0,0);
        buf
    }

    //#[test]
    fn test_move() {
        let mut buf = get_buf();
        buf.set_size(20, 10);
        buf.set_cursor(0,0);
        for command in buf.render_view() {
            println!("{:?}", command);
        }

        // move down
        buf.command(ReadEvent::MoveCursorY(1));
        assert_eq!(buf.pos().1, 1);

        //move back up
        buf.command(ReadEvent::MoveCursorY(-1));
        assert_eq!(buf.pos().1, 0);

        // try to move out of bounds
        buf.command(ReadEvent::MoveCursorY(-1));
        assert_eq!(buf.pos().1, 0);

        // try to go out of bounds at the end of the file
        buf.command(ReadEvent::MoveCursorY(100));
        assert_eq!(buf.pos().1, 10);
    }

    //#[test]
    fn test_line() {
        let mut buf = get_buf();
        buf.set_size(20, 10);
        buf.set_cursor(0,0);

        buf.command(ReadEvent::Line(-1));
        assert_eq!(buf.cursor().1, 3);
    }

}
