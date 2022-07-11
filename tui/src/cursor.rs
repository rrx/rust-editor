use super::ViewChar;
use super::*;
use ::num::Integer;
use editor_core::{nth_next_grapheme_boundary, nth_prev_grapheme_boundary, BufferConfig};
use log::*;
use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Cursor {
    pub line_inx: usize, // the line index (0 based)
    pub wraps: usize,    // number of rows when wrapped, >0
    pub lc0: usize,      // char for start of line, relative to start of file
    pub lc1: usize,      // char for end of line, relative to start of file
    pub c: usize,        // char position from start of file
    pub r: usize,        // rendered position from start of line
    pub wrap0: usize,    // current wrap
    pub x_hint: usize,
    pub line_len: usize,
    pub unicode_width: usize,
    pub line: String,
    pub config: BufferConfig,
    pub elements: ViewCharCollection, // cached line
}

pub struct WrapIndex {
    pub r0: usize, // render index for start of wrap, relative to start of line
    pub r1: usize, // render index for end of wrap, relative to start of line
    pub c0: usize, // char for start of wrap relative to start of file
    pub c1: usize, // char for end of wrap relative to start of file
    pub cx: usize, // char position relative to the start of wrap
    pub rx: usize, // rendered position from start of wrap
}

impl WrapIndex {
    fn from_cursor(cursor: &Cursor, sx: usize) -> WrapIndex {
        // render index for start and end of word wrapped line, in rendered elements
        let r0 = cursor.wrap0 * sx;
        let r1 = std::cmp::min(cursor.unicode_width, (cursor.wrap0 + 1) * sx);
        let rx = cursor.r - r0;

        let c0 = cursor.lc0 + cursor.elements.char_length_range(0, r0);
        let c1 = c0 + cursor.elements.char_length_range(r0, r1);
        let cx = cursor.c - c0;
        WrapIndex {
            r0,
            r1,
            c0,
            c1,
            cx,
            rx,
        }
    }
}

impl Cursor {
    pub fn print(&self) {
        info!("line_inx: {}, wraps: {}, lc0: {}, lc1: {}, c: {}, r: {}, wrap0: {}, x_hint: {}, line_len: {}, unicode_width: {}",
                self.line_inx,
                self.wraps,
                self.lc0,
                self.lc1,
                self.c,
                self.r,
                self.wrap0,
                self.x_hint,
                self.line_len,
                self.unicode_width
                );
    }
    pub fn simple_format(&self) -> String {
        format!(
            "(Line:{},r:{},dc:{},xh:{},w:{}/{},uw:{},cw:{})",
            self.line_inx + 1,
            self.r,
            self.c - self.lc0,
            self.x_hint,
            self.wrap0 + 1,
            self.wraps,
            self.unicode_width,
            self.line_len
        )
    }

    pub fn to_line_format(
        &self,
        config: &BufferConfig,
        sx: usize,
        highlight: String,
    ) -> Vec<LineFormat> {
        //debug!("to_line_format: {}: {:?}", self.simple_format(), sx);
        // get the current row of the wrapped line
        match format_wrapped(&self.line, sx, highlight, config).get(self.wrap0) {
            Some(row) => row.clone(),
            None => vec![],
        }
    }

    pub fn to_elements(&self, sx: usize) -> Vec<ViewChar> {
        let wi = WrapIndex::from_cursor(&self, sx);
        self.elements.elements_range(wi.r0, wi.r1)
    }

    pub fn to_string(&self, sx: usize) -> String {
        let wi = WrapIndex::from_cursor(&self, sx);
        self.line
            .graphemes(true)
            .skip(wi.c0)
            .take(wi.c1 - wi.c0)
            .collect()
    }

    pub fn save_x_hint(mut self, sx: usize) -> Cursor {
        let rx = self.rx(sx);
        info!("save_x_hint({:?})", (self.x_hint, rx, sx));
        self.x_hint = rx;
        self
    }

    pub fn rx(&self, sx: usize) -> usize {
        let r0 = self.wrap0 * sx;
        self.r - r0
    }

    // get the rendered index from the char index
    pub fn lc_to_r(&self, lc: usize) -> usize {
        self.elements.lc_to_r(lc)
    }

    pub fn r_to_c(&self, r: usize) -> usize {
        self.lc0 + self.elements.r_to_lc(r)
    }
}

use std::cmp::{Ord, Ordering};
impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.line_inx == other.line_inx && self.c == other.c
    }
}

impl Eq for Cursor {}
impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> Ordering {
        self.line_inx
            .cmp(&other.line_inx)
            .then(self.c.cmp(&other.c))
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn cursor_eof(text: &Rope, sx: usize, config: &BufferConfig) -> Cursor {
    cursor_from_char(text, sx, config, text.len_chars(), 0)
}

pub fn cursor_start(text: &Rope, sx: usize, config: &BufferConfig) -> Cursor {
    cursor_from_char(text, sx, config, 0, 0)
}

pub fn cursor_resize(text: &Rope, sx: usize, cursor: &Cursor) -> Cursor {
    cursor_from_char(text, sx, &cursor.config, cursor.c, cursor.x_hint)
}

pub fn cursor_update(text: &Rope, sx: usize, cursor: &Cursor) -> Cursor {
    cursor_resize(text, sx, cursor)
}

pub fn cursor_from_line_wrapped(
    text: &Rope,
    sx: usize,
    config: &BufferConfig,
    line_inx: i64,
) -> Cursor {
    let inx = if line_inx < 0 {
        text.len_lines() - 1 - i64::abs(line_inx) as usize
    } else {
        line_inx as usize
    };
    cursor_from_line(text, sx, config, inx)
}

pub fn cursor_from_line(text: &Rope, sx: usize, config: &BufferConfig, line_inx: usize) -> Cursor {
    let c = text.line_to_char(line_inx);
    cursor_from_char(text, sx, config, c, 0)
}

// move inside a line, with wrapping
// -1 goes to the end of the line
pub fn cursor_move_to_lc(text: &Rope, sx: usize, cursor: &Cursor, lc: i32) -> Cursor {
    let c: usize = cursor.lc0 + (lc.rem_euclid(cursor.line_len as i32)) as usize;
    debug!(
        "cursor_move_to_lc: {:?}",
        (cursor.c, cursor.lc0, cursor.line_len, lc, c)
    );
    cursor_from_char(text, sx, &cursor.config, c, cursor.x_hint)
}

pub fn cursor_char_backward(text: &Rope, sx: usize, cursor: &Cursor, dx_back: usize) -> Cursor {
    info!(
        "cursor_char_backwards: {:?}",
        (
            cursor.line_inx,
            cursor.c,
            cursor.unicode_width,
            dx_back,
            cursor.x_hint
        )
    );

    let c = nth_prev_grapheme_boundary(text.get_slice(..).unwrap(), cursor.c, dx_back);
    cursor_from_char(text, sx, &cursor.config, c, cursor.x_hint)
}

pub fn cursor_char_forward(text: &Rope, sx: usize, cursor: &Cursor, dx_forward: usize) -> Cursor {
    info!(
        "cursor_char_forward: {:?}",
        (
            cursor.line_inx,
            cursor.c,
            cursor.unicode_width,
            dx_forward,
            cursor.x_hint
        )
    );

    let mut c = nth_next_grapheme_boundary(text.get_slice(..).unwrap(), cursor.c, dx_forward);

    if c >= text.len_chars() - 1 {
        // don't go paste the end.
        // alternatively, we could wrap around to the start
        c = text.len_chars() - 1;
    }

    cursor_from_char(text, sx, &cursor.config, c, cursor.x_hint)
}

pub fn cursor_move_to_y(text: &Rope, sx: usize, cursor: &Cursor, dy: i32) -> Cursor {
    info!(
        "cursor_move_to_y:{:?}",
        (&cursor.c, dy, cursor.x_hint, &cursor.x_hint)
    );
    let mut c = cursor.clone();

    if dy > 0 {
        let mut count = 0;
        loop {
            if count >= dy {
                break;
            }
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    count += 1;
                }
                None => break,
            }
        }
    } else if dy < 0 {
        let mut count = 0;
        loop {
            if count <= dy {
                break;
            }
            match cursor_visual_prev_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    count -= 1;
                }
                None => break,
            }
        }
    }
    c
}

pub fn cursor_move_to_x(text: &Rope, sx: usize, cursor: &Cursor, dx: i32) -> Cursor {
    info!(
        "cursor_move_to_x: {:?}",
        (cursor.line_inx, cursor.r, cursor.unicode_width, dx)
    );
    let c = if dx < 0 {
        let dx_back = i32::abs(dx) as usize;
        cursor_char_backward(text, sx, cursor, dx_back)
    } else if dx > 0 {
        let dx_forward = dx as usize;
        cursor_char_forward(text, sx, cursor, dx_forward)
    } else {
        cursor.clone()
    };
    c.save_x_hint(sx)
}

pub fn cursor_to_line_relative(
    _text: &Rope,
    sx: usize,
    cursor: &Cursor,
    wrap: usize,
    rx: usize,
) -> Cursor {
    debug!("cursor_to_line_relative: {:?}", (cursor.line_inx, wrap, rx));
    let mut c = cursor.clone();
    let end;
    if c.unicode_width > 0 {
        end = c.unicode_width - 1;
    } else {
        end = 0;
    }

    let r = std::cmp::min(end, wrap * sx + rx);
    c.wrap0 = r / sx;
    c.r = r;
    c.c = c.lc0 + c.elements.r_to_lc(r);
    c.x_hint = rx;
    c
}

pub fn cursor_line_relative(
    text: &Rope,
    sx: usize,
    config: &BufferConfig,
    line_inx: usize,
    wrap: usize,
    rx: usize,
) -> Option<Cursor> {
    if line_inx >= text.len_lines() {
        return None;
    }
    let cursor = cursor_from_line(text, sx, config, line_inx);
    Some(cursor_to_line_relative(text, sx, &cursor, wrap, rx))
}

pub fn cursor_visual_prev_line(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    debug!(
        "cursor_visual_prev_line:{:?}",
        (cursor.line_inx, cursor.x_hint)
    );
    // use x_hint in this function
    let rx = cursor.x_hint;
    if cursor.wrap0 > 0 {
        Some(cursor_to_line_relative(
            text,
            sx,
            &cursor,
            cursor.wrap0 - 1,
            rx,
        ))
    } else {
        if cursor.line_inx == 0 {
            return None;
        } else {
            let c = cursor_from_line(&text, sx, &cursor.config, cursor.line_inx - 1);
            Some(cursor_to_line_relative(text, sx, &c, c.wraps - 1, rx))
        }
    }
}

pub fn cursor_visual_next_line(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    debug!(
        "cursor_visual_next_line:{:?}",
        (cursor.line_inx, cursor.x_hint)
    );
    // use x_hint in this function
    let rx = cursor.x_hint;
    let wrap = cursor.wrap0 + 1;
    if wrap < cursor.wraps {
        Some(cursor_to_line_relative(text, sx, cursor, wrap, rx))
    } else {
        let line_inx = cursor.line_inx + 1;
        if line_inx < text.len_lines() {
            cursor_line_relative(text, sx, &cursor.config, line_inx, 0, rx)
        } else {
            None
        }
    }
}

pub fn cursor_from_char(
    text: &Rope,
    sx: usize,
    config: &BufferConfig,
    mut c: usize,
    x_hint: usize,
) -> Cursor {
    //debug!("cursor_from_char: {:?}", (c, sx, x_hint));
    if c > text.len_chars() {
        c = text.len_chars();
    }
    let line_inx = text.char_to_line(c);
    let lc0 = text.line_to_char(line_inx);
    let lc1 = text.line_to_char(line_inx + 1);
    let line = text.line(line_inx).to_string();
    let elements = string_to_elements(&line, config);

    // must be >= 1
    let wraps = (elements.unicode_width()+1).div_ceil(&sx);

    let r = elements.lc_to_r(c - lc0);
    let wrap0 = r / sx;

    Cursor {
        line_inx,
        x_hint,
        c,
        r,
        wraps,
        wrap0,
        lc0,
        lc1,
        unicode_width: elements.unicode_width(),
        elements: elements.clone(),
        line_len: line.len(),
        line,
        config: config.clone(),
    }
}

struct TextIterator<'a> {
    text: &'a Rope,
    reverse: bool,
    c: usize,
}

impl<'a> Iterator for TextIterator<'a> {
    type Item = char;
    fn next(&mut self) -> Option<Self::Item> {
        if self.text.len_chars() == 0 {
            None
        } else if self.reverse {
            if self.c == 0 {
                None
            } else {
                self.c -= 1;
                let ch = self.text.char(self.c);
                Some(ch)
            }
        } else {
            if self.c >= self.text.len_chars() - 1 {
                None
            } else {
                let ch = self.text.char(self.c);
                self.c += 1;
                Some(ch)
            }
        }
    }
}

impl<'a> TextIterator<'a> {
    fn new(text: &'a Rope, c: usize, reverse: bool) -> Self {
        Self { text, reverse, c }
    }

    fn take_while1(&'a mut self, p: impl FnMut(&char) -> bool) -> &'a mut TextIterator {
        let start = self.c;
        let count = self.take_while(p).count();
        if self.reverse {
            self.c = start - count;
        } else {
            self.c = start + count;
        }
        self
    }
}

fn is_special(ch: &char) -> bool {
    ":;'\"(){}[]".contains(*ch)
}

pub fn cursor_move_to_char(
    text: &Rope,
    sx: usize,
    cursor: &Cursor,
    d: i32,
    ch: char,
    _flag: bool,
) -> Cursor {
    let start = cursor.c - cursor.lc0;
    match cursor
        .line
        .chars()
        .skip(1)
        .skip(start)
        .position(|c| c == ch)
    {
        Some(inx) => {
            debug!("cursor_move_to_char: {:?}", (d, ch, inx, start));
            cursor_move_to_lc(&text, sx, cursor, (start + inx + 1) as i32)
        }
        None => cursor.clone(),
    }
}

pub fn cursor_move_to_word(text: &Rope, sx: usize, cursor: &Cursor, d: i32, cap: bool) -> Cursor {
    if d == 0 {
        return cursor.clone();
    }
    let mut c = cursor.c;
    let mut count = 0;
    let d_abs = i32::abs(d);
    while count < d_abs {
        let mut it = TextIterator::new(text, c, d < 0);

        if d < 0 {
            let mut it2 = it.take_while1(|ch: &char| ch.is_whitespace() || is_special(ch));
            if cap {
                it2 = it2.take_while1(|ch: &char| !ch.is_whitespace());
            } else {
                it2 = it2.take_while1(|ch: &char| ch.is_alphanumeric() || is_special(ch));
            }
            c = it2.c;
        } else {
            let mut it2;
            if cap {
                it2 = it.take_while1(|ch: &char| !ch.is_whitespace());
            } else {
                it2 = it.take_while1(|ch: &char| ch.is_alphanumeric() || is_special(ch));
            }
            it2 = it2.take_while1(|ch: &char| ch.is_whitespace() || is_special(ch));
            c = it2.c;
        }
        count += 1;
    }

    cursor_from_char(text, sx, &cursor.config, c, 0).save_x_hint(sx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lineworker::*;

    #[test]
    fn test_cursor_next_visual_line() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("a\nb\nc");
        let (sx, _sy) = (10, 10);
        let c = cursor_start(&text, sx, &config);
        let c_next = cursor_visual_next_line(&text, sx, &c);
        println!("r2:{:?}", (&c, &c_next));
    }

    #[test]
    fn test_cursor_visual() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("123456789\nabcdefghijk\na\nb\nc");
        let (sx, _sy) = (5, 3);
        let c0 = cursor_from_char(&text, sx, &config, 10, 0);
        println!("c0:{:?}", (&c0.to_string(sx)));
        let mut c = cursor_start(&text, sx, &config);
        let mut i = 0;
        println!("c1:{:?}", (&c.to_string(sx)));
        loop {
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c1:{:?}", (i, &c.to_string(sx)));
                    i += 1;
                }
                None => break,
            }
        }
        loop {
            match cursor_visual_prev_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c2:{:?}", (i, &c.to_string(sx)));
                    i += 1;
                }
                None => break,
            }
        }
    }

    #[test]
    fn test_cursor_r_to_c() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("a\n12345\nc\n地球\nasdf\n");
        let (sx, _sy) = (3, 10);
        let mut cursor = cursor_start(&text, sx, &config);
        for i in 0..20 {
            let r = cursor.lc_to_r(cursor.c - cursor.lc0);
            let c = cursor.r_to_c(cursor.r);
            println!(
                "c:{:?}",
                (
                    i,
                    cursor.r,
                    r,
                    cursor.c,
                    c,
                    cursor.lc0,
                    &cursor.line,
                    text.chars_at(cursor.c).next()
                )
            );
            assert_eq!(cursor.c, c);
            assert_eq!(cursor.r, r);
            cursor = cursor_char_forward(&text, sx, &cursor, 1);
        }
    }

    #[test]
    fn test_cursor_backward() {
        let config = BufferConfig::config_for(None);
        let mut s = (0..10).map(|_| '\t').collect::<String>();
        s.push('\n');
        let text = Rope::from_str(&s);
        let (sx, _sy) = (6, 10);
        let mut cursor = cursor_eof(&text, sx, &config);
        for i in 0..20 {
            println!("c:{:?}", (i, cursor.r, cursor.c));
            cursor = cursor_char_backward(&text, sx, &cursor, 1);
        }
    }

    #[test]
    fn test_cursor_move_y() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("123456789\nabcdefghijk\na\nb\nc");
        let (sx, sy) = (5, 3);
        let mut c = cursor_start(&text, sx, &config);
        let mut start = c.clone();
        for i in 0..8 {
            let (cx, cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
            start = rows[0].clone();
            println!("current:{:?}", (i, cx, cy, &start, &c));
            rows.iter().enumerate().for_each(|(i2, row)| {
                let x;
                if cy == (i2 as u16) {
                    x = '*';
                } else {
                    x = ' ';
                }
            });
            c = cursor_move_to_y(&text, sx, &c, 1);
        }
        for i in 0..8 {
            let (cx, cy, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
            start = rows[0].clone();
            println!("current:{:?}", (i, cx, cy, &start, &c));
            rows.iter().enumerate().for_each(|(i2, row)| {
                let x;
                if cy == (i2 as u16) {
                    x = '*';
                } else {
                    x = ' ';
                }
            });
            c = cursor_move_to_y(&text, sx, &c, -1);
        }
    }

    #[test]
    fn test_cursor_move_y_2() {
        let config = BufferConfig::config_for(None);
        let text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let mut c = cursor_start(&text, sx, &config);
        let mut start = c.clone();

        // init
        let (_, _, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].clone();
        println!("r0:{:?}", (&c, &start));

        c = cursor_move_to_y(&text, sx, &c, 1);
        let (_, _, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].clone();
        println!("r1:{:?}", (&c, &start));

        c = cursor_move_to_y(&text, sx, &c, -1);
        let (_, _, rows) = LineWorker::screen_from_cursor(&text, sx, sy, &start, &c);
        start = rows[0].clone();
        println!("r2:{:?}", (&c, &start));
    }

    #[test]
    fn test_cursor_move_x_utf() {
        let config = BufferConfig::default();
        let s = "地球".to_string();
        let text = Rope::from_str(&s);
        let (sx, sy) = (10, 10);
        let mut c0 = cursor_start(&text, sx, &config);
        println!("0:{:?}", (c0));
        let c1 = cursor_move_to_x(&text, sx, &c0, 1);
        println!("0:{:?}", (c1));
    }

    #[test]
    fn test_cursor_move_x_control() {
        let config = BufferConfig::default();
        let mut s = String::from("");
        s.push(char::from_u32(13).unwrap());
        s.push(char::from_u32(13).unwrap());
        let text = Rope::from_str(&s);
        let (sx, sy) = (10, 10);
        let mut c0 = cursor_start(&text, sx, &config);
        println!("0:{:?}", (c0));
        let c1 = cursor_move_to_x(&text, sx, &c0, 1);
        println!("0:{:?}", (c1));
    }
}
