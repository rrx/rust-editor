use log::*;
use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use crate::frontend::DrawCommand;
use crate::ism::{Mode, Command};
use std::convert::TryInto;
use super::*;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Wrap {
    pub lc0: usize,
    pub lc1: usize,
    pub c0: usize,
    pub c1: usize,
    pub offset: usize,
    pub wrap0: usize,
    pub wrap1: usize,
    pub line0: usize,
    pub line1: usize,
    pub wraps: usize,
    pub dirty: bool
}

impl Wrap {
    pub fn to_string(&self, buf: &TextBuffer, translate: bool) -> String {
        let mut s = buf.text.slice(self.c0..self.c1).to_string();
        if translate {
            s = s.replace("\t", "...-");
        }
        s
    }
}

#[derive(Debug)]
pub struct SmartBuffer<'a> {
    text: Rope,
    path: &'a str,
    dirty: bool,
    tab_size: usize,
}

impl<'a> SmartBuffer<'a> {
    pub fn new(text: Rope, path: &'a str) -> Self {
        Self {
            text: text,
            path: path,
            dirty: false,
            tab_size: 4,
        }
    }

    pub fn from_path(path: &'a str) -> io::Result<Self> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(Self::new(text, path))
    }

    pub fn from_str(s: &str) -> Self {
        let text = Rope::from_str(s);
        Self::new(text, "")
    }

    pub fn insert_char(&mut self, c: usize, ch: char) {
        self.dirty = true;
        self.text.insert_char(c, ch);
    }

    pub fn to_string(&self, start: usize, end: usize, translate: bool) -> String {
        let mut s = self.text.slice(start..end).to_string();
        if translate {
            s = s.replace("\t", "...-");
        }
        s
    }

    // create a Wrap object given the current position and the width of the viewport
    pub fn char_to_wrap(&self, sx: u16, c: usize) -> Option<Wrap> {
        let vsx = sx as usize;
        let text = &self.text;
        let len_chars = text.len_chars();
        if c >= len_chars && len_chars > 0 {
            self.char_to_wrap(sx, len_chars-1)
        } else {
            let line = text.char_to_line(c);
            let lc0 = text.line_to_char(line);
            let lc1 = text.line_to_char(line+1);
            let translated = self.text.slice(lc0..lc1).to_string().replace("\t", "...-");
            let wrap0 = (c - lc0) / vsx;
            let c0 = lc0 + wrap0 * vsx;
            let mut wrap1 = wrap0 + 1;
            let wraps = translated.len() / vsx + 1;
            let c1;
            if wrap1 == wraps {
                c1 = lc1;
                wrap1 = 0;
            } else {
                c1 = c0 + vsx;
            }
            Some(Wrap {
                lc0: lc0,
                lc1: lc1,
                c0: c0,
                c1: c1,
                offset: c - c0,
                wrap0: wrap0,
                wrap1: wrap1,
                line0: line,
                line1: line+1,
                wraps: wraps,
                dirty: true
            })
        }
    }

    pub fn line_to_wrap(&self, sx: u16, line: usize) -> Option<Wrap> {
        let len_lines = self.text.len_lines();
        if line >= len_lines {
            None
        } else {
            let c = self.text.line_to_char(line);
            self.char_to_wrap(sx, c)
        }
    }

    pub fn prev_wrap(&self, sx: u16, w: &Wrap) -> Option<Wrap> {
        if w.wrap0 > 0 {
            let c0 = w.lc0 + (w.wrap0-1) * sx as usize;
            self.char_to_wrap(sx, c0)
        } else if w.line0 > 0 {
            let offset = w.offset;
            let nw = self.char_to_wrap(sx, w.lc0-1);
            if let Some(mut w0) = nw {
                w0.offset = offset;
                Some(w0)
            } else {
                nw
            }
        } else {
            None
        }
    }

    pub fn next_wrap(&self, sx: u16, w:  &Wrap) -> Option<Wrap> {
        let len_chars = self.text.len_chars();
        if w.c1 >= len_chars {
            None
        } else {
            self.char_to_wrap(sx, w.c1)
        }
    }

    pub fn delta_wrap(&self, sx: u16, c: usize, dy: i32) -> Wrap {
        let start = c;
        let mut w = self.char_to_wrap(sx, start).unwrap();

        if dy > 0 {
            let mut count = dy;
            while count > 0 {
                match self.next_wrap(sx, &w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }

        if dy < 0 {
            let mut count = (-dy) as usize;
            while count > 0 {
                match self.prev_wrap(sx, &w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }
        w
    }

    pub fn wrap_window(&self, sx: u16, c: usize, size: usize, reverse: bool) -> Vec<Wrap> {
        let mut out = Vec::new();
        let ow = self.char_to_wrap(sx, c);

        let r;
        if reverse {
            r = -1;
        } else {
            r = 1;
        }

        if ow.is_some() && size > 0 {
            let mut w = ow.unwrap();
            out.push(w);

            let mut count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(sx, c, r*count);
                if w0.c0 == w.c0 {
                    break;
                }
                out.push(w0);
                w = w0;
                count += 1;
            }

            w = ow.unwrap();
            count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(sx, c, -r*count);
                if w0.c0 == w.c0 {
                    break;
                }
                out.insert(0,w0);
                w = w0;
                count += 1;
            }
        }
        if reverse {
            out.reverse();
        }
        out
    }

    pub fn wrap_to_string(&self, w: &Wrap, translate: bool) -> String {
        let mut s = self.text.slice(w.c0..w.c1).to_string();
        if translate {
            s = s.replace("\t", "...|")
        }
        s
    }

    pub fn scroll(&mut self, spec: &ViewSpec, port: &ViewPort, y: i32) -> usize {
        let w = self.delta_wrap(spec.sx, port.char_start, y);
        w.c0
    }

    pub fn line_move(&self, sx: u16, c: usize, x: i32) -> usize {
        let mut w = self.char_to_wrap(sx, c).unwrap();
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
        c
    }

    pub fn jump_to_line(&mut self, line: i64) -> usize {
        // 0 is the start
        // negative lines is the number of lines from the end of the file
        let lines: usize = self.text.len_lines() - 1;
        let mut current = line as usize;
        if line < 0 {
            current = lines - i64::abs(line) as usize;
        }

        if current > lines {
            current = lines;
        }

        self.text.line_to_char(current)
        //let w = self.line_to_wrap(sx, current).unwrap();
        //w.c0
    }

    pub fn move_cursor_y(&mut self, sx: u16, c0: usize, cursor: &ViewCursor, dy: i32) -> usize {
        let w = self.delta_wrap(sx, c0, dy);

        // use x hint
        let mut c = w.c0 + cursor.x_hint() as usize;
        if c >= w.lc1 && w.lc0 < w.lc1 {
            c = w.lc1 - 1;
        }
        c
    }

    pub fn move_cursor_x(&mut self, sx: u16, c0: usize, dx: i32) -> (usize, usize) {
        self._move_cursor_x(sx, c0, dx, false)
    }

    pub fn _move_cursor_x(&mut self, sx: u16, c0: usize, dx: i32, constrain: bool) -> (usize, usize) {
        let mut c = c0 as i32 + dx;
        if c < 0 {
            c = 0;
        } else if c > self.text.len_chars() as i32 {
            c = self.text.len_chars() as i32;
        }

        let mut c1 = c as usize;
        if constrain {
            // restrict x movement to the specific line
            let mut w = self.char_to_wrap(sx, c0).unwrap();
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

        let mut w = self.char_to_wrap(sx, c1).unwrap();
        let hint = c1 - w.c0;
        (c1, hint)
        //if c0 != c1 {
            //self.view.cursor_x_hint = hint as u16;
            //self.update_window(c1);
        //}
    }

}

