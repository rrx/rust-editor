use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

use crate::frontend::{DrawCommand, ReadEvent};

pub enum EditMode {
    Insert,
    Normal,
    Command
}

pub struct TextBuffer {
    text: Rope,
    path: String,
    dirty: bool,
    line_offset: usize,
    mode: EditMode,
    view: EditorView
}

pub struct EditorView {
    size: (u16, u16),
    cursor: (u16, u16)
}

impl TextBuffer {
    pub fn from_path(path: &str) -> io::Result<TextBuffer> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(TextBuffer {
            text: text,
            path: path.to_string(),
            dirty: false,
            line_offset: 0,
            mode: EditMode::Normal,
            view: EditorView { size: (0,0), cursor: (0,0) }
        })
    }

    pub fn from_str(s: &str) -> Self {
        let text = Rope::from_str(s);
        Self {
            text: text,
            path: "".to_string(),
            dirty: false,
            line_offset: 0,
            mode: EditMode::Normal,
            view: EditorView { size: (0,0), cursor: (0,0) }
        }
    }

    pub fn set_mode(&mut self, mode: EditMode) {
        self.mode = mode;
    }

    pub fn set_size(&mut self, x: u16, y: u16) {
        self.view.size = (x, y);
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
        if self.line_offset < 0 {
            self.line_offset = 0;
        }
        if self.line_offset >= self.text.len_lines() {
            self.line_offset = self.text.len_lines() - 1;
        }
    }

    pub fn command(&mut self, evt: ReadEvent) {
        match evt {
            ReadEvent::MoveCursor(dx, dy) => {
                let cursor_y = self.view.cursor.1 as i32 + dy;
                if cursor_y < 0 {
                    self.view.cursor.1 = 0;
                    let dline = i32::abs(cursor_y) as usize;
                    if dline < self.line_offset {
                        self.line_offset -= dline;
                    } else {
                        self.line_offset = 0;
                    }
                } else if cursor_y > self.view.size.1 as i32 {
                    self.view.cursor.1 = self.view.size.1;
                    self.line_offset += cursor_y as usize;
                } else {
                    self.view.cursor.1 = cursor_y as u16;
                }
                self.normalize();
            }
            ReadEvent::Stop => (),
            ReadEvent::Scroll(dy) => {
                let mut offset: i32 = self.line_offset as i32;
                offset += dy as i32;
                if offset < 0 {
                    offset = 0;
                } else if offset >= self.text.len_lines() as i32 {
                    offset = self.text.len_lines() as i32 - 1;
                }
                self.line_offset = offset as usize;
            }

            // Goto a line
            ReadEvent::Line(line) => {
                let offset: usize;
                if line < 0 {
                    offset = self.text.len_lines() - i64::abs(line) as usize;
                } else {
                    offset = line as usize;
                }

                let lines: usize = self.text.len_lines();
                // negative lines is the number of lines from the end of the file
                if self.view.size.1 as usize >= lines {
                    self.line_offset = 0;
                    self.set_cursor(0,offset as u16);

                // handle case where we are at the end of the file
                } else if lines - offset < self.view.size.1 as usize {
                    self.line_offset = lines - self.view.size.1 as usize;
                    self.set_cursor(0, self.view.size.1 - (lines - offset) as u16);

                // else somewhere in the middle of the file
                } else {
                    self.line_offset = offset;
                    self.set_cursor(0,0);
                }
            }

            ReadEvent::Resize(a, b) => {
                self.set_size(a, b);
            }

            ReadEvent::Mouse(x, y) => {
                self.set_cursor(x, y);
            }
        }
    }

    pub fn generate_commands(&self) -> Vec<DrawCommand> {
        let mut out = Vec::new();

        let mut row: u16 = 0;
        let mut lines = self.lines_at(self.line_offset);

        let (sx, sy) = self.view.size;

        while row < sy {
            match lines.next() {
                Some(line) => {
                    let mut start = 0;
                    let len = line.len_chars();
                    // handle line wrapping
                    while start < len {
                        let mut s = String::with_capacity(sx as usize);
                        let end = start + std::cmp::min(len-start, sx as usize);
                        //println!("start: {}, end: {}, sx: {}, row: {}, len: {}", start, end, sx, row, len);

                        if end > start {
                            let s0 = line.slice(start..end);
                            s.insert_str(0, &format!("{}", s0).to_owned());
                            out.push(DrawCommand::Line(row, s.replace("\n", ".")));
                            start = end;
                            row += 1;
                        }
                    }
                },
                None => {
                    out.push(DrawCommand::Line(row, ";".to_string()));
                    row += 1;
                }
            }
        }
        let p = self.cursor();
        out.push(DrawCommand::Cursor(p.0, p.1));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_buf() -> TextBuffer {
        TextBuffer::from_str(r###"test
estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst estst
asdf
"###)
    }
    #[test]
    fn test_move() {
        let mut buf = get_buf();
        buf.set_size(20, 10);
        buf.set_cursor(0,0);
        for command in buf.generate_commands() {
            println!("{:?}", command);
        }

        // move down
        buf.command(ReadEvent::MoveCursor(0,1));
        assert_eq!(buf.pos().1, 1);

        //move back up
        buf.command(ReadEvent::MoveCursor(0,-1));
        assert_eq!(buf.pos().1, 0);

        // try to move out of bounds
        buf.command(ReadEvent::MoveCursor(0,-1));
        assert_eq!(buf.pos().1, 0);

        // try to go out of bounds at the end of the file
        buf.command(ReadEvent::MoveCursor(0,100));
        assert_eq!(buf.pos().1, 10);
    }

    #[test]
    fn test_line() {
        let mut buf = get_buf();
        buf.set_size(20, 10);
        buf.set_cursor(0,0);

        buf.command(ReadEvent::Line(-1));
        assert_eq!(buf.cursor().1, 3);
    }

}
