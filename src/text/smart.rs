use log::*;
use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use crate::frontend::DrawCommand;
use crate::ism::{Mode, Command};
use crate::text::wrap::WrapValue;
use std::convert::TryInto;

#[derive(Debug)]
pub struct SmartBuffer<'a> {
    text: Rope,
    path: &'a str,
    dirty: bool
}

impl<'a> SmartBuffer<'a> {
    pub fn new(text: Rope, path: &'a str) -> Self {
        Self {
            text: text,
            path: path,
            dirty: false
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

    // create a Wrap object given the current position and the width of the viewport
    pub fn char_to_wrap(&self, c: usize, sx: usize) -> Option<WrapValue> {
        let text = &self.text;
        let len_chars = text.len_chars();
        if c >= len_chars && len_chars > 0 {
            self.char_to_wrap(len_chars-1, sx)
        } else {
            let line = text.char_to_line(c);
            let lc0 = text.line_to_char(line);
            let lc1 = text.line_to_char(line+1);
            let wrap0 = (c - lc0) / sx;
            let c0 = lc0 + wrap0 * sx;
            let mut wrap1 = wrap0 + 1;
            let wraps = (lc1 - lc0) / sx + 1;
            let c1;
            if wrap1 == wraps {
                c1 = lc1;
                wrap1 = 0;
            } else {
                c1 = c0 + sx;
            }
            Some(WrapValue {
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

    pub fn prev_wrap(&self, w: &WrapValue, sx: usize) -> Option<WrapValue> {
        if w.wrap0 > 0 {
            let c0 = w.lc0 + (w.wrap0-1) * sx;
            self.char_to_wrap(c0, sx)
        } else if w.line0 > 0 {
            let offset = w.offset;
            let nw = self.char_to_wrap(w.lc0-1, sx);
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

    pub fn next_wrap(&self, w:  &WrapValue, sx: usize) -> Option<WrapValue> {
        let len_chars = self.text.len_chars();
        if w.c1 >= len_chars {
            None
        } else {
            self.char_to_wrap(w.c1, sx)
        }
    }

    pub fn delta_wrap(&self, c: usize, dy: i32, sx: usize) -> WrapValue {
        let start = c;
        let mut w = self.char_to_wrap(start, sx).unwrap();

        if dy > 0 {
            let mut count = dy;
            while count > 0 {
                match self.next_wrap(&w, sx) {
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
                match self.prev_wrap(&w, sx) {
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

    pub fn wrap_window(&self, c: usize, size: usize, reverse: bool, sx: usize) -> Vec<WrapValue> {
        let mut out = Vec::new();
        let ow = self.char_to_wrap(c, sx);

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
                let w0 = self.delta_wrap(c, r*count, sx);
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
                let w0 = self.delta_wrap(c, -r*count, sx);
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

    pub fn wrap_to_string(&self, w: &WrapValue) -> String {
        self.text.slice(w.c0..w.c1).to_string()
    }

    pub fn line_move(&self, c: usize, sx: usize, x: i32) -> usize {
        let mut w = self.char_to_wrap(c, sx).unwrap();
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
}

