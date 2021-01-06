use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

use crate::frontend::{DrawCommand, ReadEvent};

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
    pub char_start: usize,
    //char_range: (usize, usize),
    //line_range: (usize, usize),
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
    pub debug: String
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
            debug: String::new()
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

        if self.line_offset < 0 {
            self.line_offset = 0;
        }
        if self.line_offset >= line_count {
            self.line_offset = line_count - 1;
        }

        if self.line_current < 0 {
            self.line_current = 0;
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
            }
            ReadEvent::MoveCursorY(dy) => {
                let line_count = self.text.len_lines() as i32 - 1;
                // calculate the line we are moving to
                let mut current: i32 = self.line_current as i32 + dy;
                if current >= line_count {
                    // beyond end
                    current = line_count - 1;
                } else if current < 0 {
                    // beyond beginning
                    current = 0;
                }
                let current_dy = current - self.line_current as i32;

                let mut offset = self.line_offset;
                let mut cursor;
                if current < self.line_offset as i32 {
                    // scroll up
                    offset = current as usize;
                    cursor = 0;
                } else if current >= (self.line_offset as i32 + self.view.vsy as i32) {
                    // scroll down
                    offset = current as usize - self.view.vsy as usize;
                    cursor = self.view.vsy as i32 - 1;
                } else {
                    // no scroll
                    cursor = self.view.cursor.1 as i32 + current_dy;
                }

                self.line_current = current as usize;
                self.line_offset = offset;
                self.view.cursor.1 = cursor as u16;
                self.view.debug = format!("A: {}/{}/{}/{}", offset, cursor, current_dy, line_count);

                //let mut cursor = self.view.cursor.1 as i32 + current_dy;
                //let mut offset_dy = 0;
                //if cursor < 0 {
                    //// scroll up
                    //offset_dy = cursor;
                    //cursor = 0;
                //} else if cursor >= self.view.vsy as i32 {
                    //// scroll down
                    //offset_dy = cursor - self.view.vsy as i32;
                    //cursor = self.view.vsy as i32 - 1;
                //}
                //let offset: i32 = self.line_offset as i32 + offset_dy;
                //let cursor_dy = cursor - self.view.cursor.1 as i32;

                //self.line_offset = offset as usize;
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
                // negative lines is the number of lines from the end of the file
                let lines: usize = self.text.len_lines();
                let current: usize;
                let mut offset: usize;
                if line < 0 {
                    current = lines - i64::abs(line) as usize;
                } else {
                    current = line as usize;
                }
                // make them the same for now and adjust offset later
                offset = current;

                if self.view.vsy as usize >= lines {
                    // case where we have more lines than fill the viewport
                    self.line_offset = 0;
                    //self.set_cursor(0,offset as u16);

                // handle case where we are at the end of the file
                } else if lines - offset < self.view.vsy as usize {
                    offset = lines - self.view.vsy as usize;
                    //self.set_cursor(0, self.view.vsy - (lines - offset) as u16);

                // else somewhere in the middle of the file
                } else {
                    //self.set_cursor(0,0);
                }
                self.line_offset = offset;
                self.line_current = current;
            }

            ReadEvent::Resize(a, b) => {
                self.set_size(a, b);
            }

            ReadEvent::Mouse(x, y) => {
                self.set_cursor(x, y);
            }
        }
    }

    pub fn render_view(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        let (sx, sy) = self.view.size;

        // don't do anything if the view port is too small
        if sy <= 3 || sx < 6 {
            return out;
        }

        out.append(&mut self.render_lines());
        out.push(DrawCommand::Status(self.view.rInfo,format!("I: {}", self.view.debug)));
        out.push(DrawCommand::Status(self.view.rCmd, "".to_string()));
        let p = self.cursor();
        out.push(DrawCommand::Cursor(p.0, p.1));
        out
    }


    pub fn render_lines(&mut self) -> Vec<DrawCommand> {
        let mut wrapped_lines = Vec::new();
        let mut c = self.char_start;
        let line = self.text.char_to_line(c);
        let lc0 = self.text.line_to_char(line);
        let vsx = self.view.vsx as usize;
        let vsy = self.view.vsy as usize;

        // normalize the char start
        // get the index of the starting row of the first line
        println!("{}/{}", lc0, c);
        let mut line_start_wrap = (c - lc0) / vsx;
        let c = line_start_wrap * vsx;
        let mut row: u16 = 0;
        // fill up the available size in the viewport
        let mut lines = self.lines_at(line);
        let mut line_count: usize = self.line_offset;
        let (sx, sy) = self.view.size;

        while wrapped_lines.len() < vsy {
            match lines.next() {
                Some(line) => {
                    let mut start = 0;
                    let len = line.len_chars();
                    // handle line wrapping
                    let mut line_count_save = line_count + 1;
                    let mut wrap = 0;
                    while start < len {
                        let mut s = String::with_capacity(sx as usize);
                        let end = start + std::cmp::min(len-start, sx as usize);
                        if end > start {
                            if line_start_wrap <= wrap {
                                let s0 = line.slice(start..end);
                                s.insert_str(0, &format!("{}", s0).to_owned());
                                wrapped_lines.push(
                                    DrawCommand::Line(row, line_count_save, s.replace("\n", ".")));
                                line_count_save = 0;
                            }
                            start = end;
                            wrap += 1;
                            row += 1;
                        }
                    }
                    // reset the start_wrap
                    line_start_wrap = 0;
                    line_count += 1;
                }
                None => {
                    wrapped_lines.push(DrawCommand::Status(row, ";".to_string()));
                    row += 1;
                }
            }
        }
        wrapped_lines
    }

    pub fn render_view2(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();

        let mut row: u16 = 0;
        let mut lines = self.lines_at(self.line_offset);

        let (sx, sy) = self.view.size;

        // don't do anything if the view port is too small
        if sy <= 3 || sx < 6 {
            return out;
        }


        let mut line_count: usize = self.line_offset;
        while row < self.view.vsy {
            match lines.next() {
                Some(line) => {
                    let mut start = 0;
                    let len = line.len_chars();
                    // handle line wrapping
                    let mut line_count_save = line_count + 1;
                    while start < len {
                        let mut s = String::with_capacity(sx as usize);
                        let end = start + std::cmp::min(len-start, sx as usize);
                        //println!("start: {}, end: {}, sx: {}, row: {}, len: {}", start, end, sx, row, len);

                        if end > start {
                            let s0 = line.slice(start..end);
                            s.insert_str(0, &format!("{}", s0).to_owned());
                            out.push(DrawCommand::Line(row, line_count_save, s.replace("\n", ".")));
                            line_count_save = 0;
                            start = end;
                            row += 1;
                        }
                    }
                    line_count += 1;
                },
                None => {
                    out.push(DrawCommand::Status(row, ";".to_string()));
                    row += 1;
                }
            }
        }
        out.push(DrawCommand::Status(self.view.rInfo,format!("I: {}", self.view.debug)));
        out.push(DrawCommand::Status(self.view.rCmd, "".to_string()));

        let p = self.cursor();
        out.push(DrawCommand::Cursor(p.0, p.1));
        out
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

    fn dump(buf: &mut TextBuffer) {
        for command in buf.render_view() {
            println!("{:?}", command);
        }
        println!("{:?}", buf);
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
