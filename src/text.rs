use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

use crate::frontend::{DrawCommand, ReadEvent};

pub struct TextBuffer {
    text: Rope,
    path: String,
    dirty: bool,
    row: u16,
    col: u16,
    line_offset: usize
}

impl TextBuffer {
    pub fn from_path(path: &str) -> io::Result<TextBuffer> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(TextBuffer {
            text: text,
            path: path.to_string(),
            dirty: false,
            row: 1,
            col: 1,
            line_offset: 0
        })
    }

    fn update_cursor(&mut self, row: u16, col: u16) {
        self.row = row;
        self.col = col;
    }

    fn pos(&self) -> (u16, u16) {
        return (self.row, self.col);
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

    pub fn handle_event(&mut self, evt: ReadEvent) {
        match evt {
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
            ReadEvent::Line(line) => {
                if line < 0 {
                    self.line_offset = self.text.len_lines() - i64::abs(line) as usize;
                } else {
                    self.line_offset = line as usize - 1;
                }
            }

            ReadEvent::Resize(a, b) => {
            }

            ReadEvent::Mouse(x, y) => {
                self.update_cursor(x, y);
            }
        }
    }

    pub fn generate_commands(&self, sx: u16, sy: u16) -> Vec<DrawCommand> {
        let mut out = Vec::new();

        let mut row: u16 = 1;
        let mut lines = self.lines_at(self.line_offset);

        while row as usize <= sy as usize {
            match lines.next() {
                Some(line) => {
                    let mut start = 0;
                    let len = line.len_chars();
                    // handle line wrapping
                    while start < len {
                        let mut s = String::with_capacity(sx as usize);
                        let end = start + std::cmp::min(len-start, sx as usize);
                        //println!("start: {}, end: {}, sx: {}, row: {}, len: {}", start, end, sx, row, len);

                        if (end > start) {
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
        let p = self.pos();
        out.push(DrawCommand::Cursor(p.0, p.1));

        out
    }

}


