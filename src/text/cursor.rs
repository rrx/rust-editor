use super::ViewChar::{self, *};
use log::*;
use ropey::Rope;
use num::Integer;

#[derive(Debug)]
pub struct RowItem {
    pub elements: Vec<ViewChar>,
    pub cursor: Cursor
}
impl RowItem {
    pub fn to_string(&self) -> String {
        use ViewChar::*;
        self.elements.iter().map(|c| {
            match c {
                NOP => ' ',
                Tab => '^',
                NL => 'v',
                Char(x) => *x,
                OOB => 'O'
            }
        }).collect::<String>()
    }
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub line_inx: usize,
    pub wraps: usize, // number of rows when wrapped
    pub lc0: usize,  // char for start of line, relative to start of file
    pub lc1: usize,  // char for end of line, relative to start of file
    pub c: usize,  // char position from start of file
    pub r: usize,  // rendered position from start of line
    pub wrap0: usize,  // current wrap
    pub x_hint: usize,
    //pub r0: usize, // render index for start of wrap, relative to start of line
    //pub r1: usize, // render index for end of wrap, relative to start of line
    //pub c0: usize, // char for start of wrap relative to start of file
    //pub c1: usize, // char for end of wrap relative to start of file
    //pub cx: usize, // char position relative to the start of wrap
    //pub rx: usize, // rendered position from start of wrap
    pub line: String,
    pub elements: Vec<ViewChar> // cached line
    //pub elements: &'a [ViewChar]
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
    let r1 = std::cmp::min(cursor.elements.len(), (cursor.wrap0+1) * sx);
    let rx = cursor.r - r0;

    let c0 = cursor.lc0 + cursor.elements.as_slice()[..r0].iter().filter(|&ch| ch != &NOP).count();
    let c1 = c0 + cursor.elements.as_slice()[r0..r1].iter().filter(|&ch| ch != &NOP).count();
    let cx = cursor.c - c0;
    //info!("char:{:?}", (cursor.c, c0, r0, r1, rx));
    WrapIndex { r0, r1, c0, c1, cx, rx }
    }
}

impl Cursor {
    pub fn simple_format(&self) -> String {
        format!("(Line:{},rx:{},dc:{},xh:{})", self.line_inx, self.r, self.c - self.lc0, self.x_hint)
    }
    pub fn to_elements(&self, sx: usize) -> Vec<ViewChar> {
        let wi = WrapIndex::from_cursor(&self, sx);
        self.elements.as_slice()[wi.r0..wi.r1].iter().cloned().collect()
    }
    pub fn to_string(&self, sx: usize) -> String {
        let wi = WrapIndex::from_cursor(&self, sx);
        self.line.chars().skip(wi.c0).take(wi.c1-wi.c0).collect()
    }

    pub fn save_x_hint(&mut self, sx: usize) {
        let rx = self.rx(sx);
        self.x_hint = rx;
    }

    pub fn rx(&self, sx: usize) -> usize {
        let r0 = self.wrap0 * sx;
        self.r - r0
    }

    // get the rendered index from the char index
    pub fn lc_to_r(&self, lc: usize) -> usize {
        //let number_of_tabs = self.line.chars().take(c-self.lc0).filter(|&ch| ch == '\t').count();
        //let r = c - self.lc0 + 4 * number_of_tabs;
        //r
        Self::line_lc_to_r(&self.line, lc)
    }

    pub fn line_r_to_lc(elements: &[ViewChar], r: usize) -> usize {
        elements.iter().take(r).fold(0, |lc, ch| {
            match ch {
                NOP => lc,
                _ => lc + 1
            }
        })
    }

    pub fn line_lc_to_r(line: &String, lc: usize) -> usize {
        line.chars().take(lc).fold(0, |r, ch| {
            match ch {
                '\t' => r + 4,
                _ => r + 1
            }
        })
    }

    pub fn r_to_c(&self, r: usize) -> usize {
        self.lc0 + Self::line_r_to_lc(&self.elements, r)
        //self.lc0 + self.elements.as_slice()[..r]
            //.iter().filter(|&ch| ch != &ViewChar::NOP).count()

         //get a rendered character that isn't a nop
        //let (r0, _) = self.elements.as_slice()[..r]
            //.iter().rev().enumerate()
            //.skip_while(|&(_, ch)| ch == &ViewChar::NOP).next().unwrap_or((0, &ViewChar::NOP));
         //count rendered characters that are not nops
        //self.lc0 + self.elements.as_slice()[..r0]
            //.iter().filter(|&ch| ch != &ViewChar::NOP).count()
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
        self.line_inx.cmp(&other.line_inx).then(self.c.cmp(&other.c))
    }
}
impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn cursor_eof(text: &Rope, sx: usize) -> Cursor {
    cursor_from_char(text, sx, text.len_chars(), 0)
}

pub fn cursor_start(text: &Rope, sx: usize) -> Cursor {
    cursor_from_char(text, sx, 0, 0)
}

pub fn cursor_from_line_wrapped(text: &Rope, sx: usize, line_inx: i64) -> Cursor {
    let mut inx;
    if line_inx < 0 {
        inx = text.len_lines() - 1 - i64::abs(line_inx) as usize;
    } else {
        inx = line_inx as usize;
    }
    cursor_from_line(text, sx, inx)
}

pub fn cursor_from_line(text: &Rope, sx: usize, line_inx: usize) -> Cursor {
    let c = text.line_to_char(line_inx);
    cursor_from_char(text, sx, c, 0)
}

pub fn cursor_to_row(cursor: &Cursor, sx: usize) -> RowItem {
    RowItem { elements: cursor.to_elements(sx), cursor: cursor.clone() }
}

pub fn cursor_move_to_lc(text: &Rope, sx: usize, cursor: &Cursor, lc: i32) -> Cursor {
    let c: usize = cursor.lc0 + (lc.rem_euclid(cursor.line.len() as i32)) as usize;
    debug!("cursor_move_to_lc: {:?}", (cursor.c, cursor.lc0, cursor.line.len(), lc, c));
    cursor_from_char(text, sx, c, cursor.x_hint)
}

fn cursor_to_line_x(text: &Rope, sx: usize, cursor: &Cursor, x: i32) -> Cursor {
    // modulus handles the wrapping very well
    let line_x: usize = (x.rem_euclid(cursor.elements.len() as i32)) as usize;
    let wrap0 = line_x / sx;
    let rx = line_x % sx;
    debug!("cursor_to_line_x: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), x, wrap0, rx));
    cursor_to_line_relative(text, sx, cursor, wrap0, rx)
}

pub fn cursor_char_backward(text: &Rope, sx: usize, cursor: &Cursor, dx_back: usize) -> Cursor {
    debug!("cursor_char_backwards: {:?}", (cursor.line_inx, cursor.c, cursor.elements.len(), dx_back));
    let dx;
    if dx_back > cursor.c {
        dx = cursor.c;
    } else {
        dx = dx_back;
    }
    let c = cursor.c - dx;
    cursor_from_char(text, sx, c, cursor.x_hint)
}

pub fn cursor_char_forward(text: &Rope, sx: usize, cursor: &Cursor, dx_forward: usize) -> Cursor {
    debug!("cursor_char_forward: {:?}", (cursor.line_inx, cursor.c, cursor.elements.len(), dx_forward));
    let mut c = cursor.c + dx_forward;
    if c >= text.len_chars() {
        c = text.len_chars() - 1;
    }
    cursor_from_char(text, sx, c, cursor.x_hint)
}

fn cursor_render_backward(text: &Rope, sx: usize, cursor: &Cursor, dx_back: usize) -> Cursor {
    debug!("cursor_render_backwards: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), dx_back));
    if dx_back <= cursor.r {
        let x = cursor.r - dx_back;
        cursor_to_line_x(text, sx, cursor, x as i32)
    } else {
        if cursor.line_inx > 0 {
            let mut remainder = dx_back - cursor.r;
            let line_inx = cursor.line_inx - 1;
            let prev = cursor_from_line(text, sx, line_inx);
            let prev2 = cursor_to_line_x(text, sx, &prev, -1); // goto end of line
            remainder -= 1;
            cursor_move_to_x(text, sx, &prev2, -1 * remainder as i32)
        } else {
            cursor_to_line_x(text, sx, &cursor, 0) // goto the start of the file
        }
    }
}

fn cursor_render_forward(text: &Rope, sx: usize, cursor: &Cursor, dx_forward: usize) -> Cursor {
    debug!("cursor_render_forward: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), dx_forward));
    let remainder = cursor.elements.len() - cursor.r;
    if remainder <= dx_forward {
        let line_inx = cursor.line_inx + 1;
        if line_inx >= text.len_lines() - 1 {
            cursor_to_line_x(text, sx, cursor, -1) // go to the end of the line if this is the last line
        } else {
            let next = cursor_from_line(text, sx, cursor.line_inx + 1);
            cursor_move_to_x(text, sx, &next, (dx_forward - remainder) as i32)
        }
    } else {
        let x = cursor.r + dx_forward;
        cursor_to_line_x(text, sx, cursor, x as i32)
    }
}


pub fn cursor_move_to_x(text: &Rope, sx: usize, cursor: &Cursor, dx: i32) -> Cursor {
    debug!("cursor_move_to_x: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), dx));
    if dx < 0 {
        let dx_back = i32::abs(dx) as usize;
        cursor_char_backward(text, sx, cursor, dx_back)
    } else if dx > 0 {
        let dx_forward = dx as usize;
        cursor_char_forward(text, sx, cursor, dx_forward)
    } else {
        cursor.clone()
    }
}

pub fn cursor_to_line_relative(text: &Rope, sx: usize, cursor: &Cursor, wrap: usize, rx: usize) -> Cursor {
    debug!("cursor_to_line_relative: {:?}", (cursor.line_inx, wrap, rx));
    let mut c = cursor.clone();
    let r = std::cmp::min(c.elements.len() - 1, wrap * sx + rx);
    c.wrap0 = r / sx;
    c.r = r;
    c.c = c.lc0 + Cursor::line_r_to_lc(&c.elements, r);
    c
}

pub fn cursor_line_relative(text: &Rope, sx: usize, line_inx: usize, wrap: usize, rx: usize) -> Option<Cursor> {
    //println!("cursor_line_relative:{:?}", (line_inx, wrap, rx));
    if line_inx >= text.len_lines() {
        return None
    }
    let cursor = cursor_from_line(text, sx, line_inx);
    //println!("line_relative:{:?}", (cursor));
    Some(cursor_to_line_relative(text, sx, &cursor, wrap, rx))
}

pub fn cursor_visual_prev_line(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    info!("cursor_visual_prev_line:{:?}", (cursor.line_inx, cursor.x_hint));
    // use x_hint in this function
    //let r0 = cursor.wrap0 * sx;
    //let rx = cursor.r - r0;
    let rx = cursor.x_hint;
    if cursor.wrap0 > 0 {
        //println!("cursor_visual_prev_line:{:?}", (cursor.line_inx, cursor.wrap0, cursor.rx));
        Some(cursor_to_line_relative(text, sx, &cursor, cursor.wrap0 - 1, rx))
    } else {
        if cursor.line_inx == 0 {
            return None;
        } else {
            let c = cursor_from_line(&text, sx, cursor.line_inx - 1);
            Some(cursor_to_line_relative(text, sx, &c, c.wraps - 1, rx))
        }
    }
}

pub fn cursor_visual_next_line(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    debug!("cursor_visual_next_line:{:?}", (cursor.line_inx, cursor.x_hint));
    // use x_hint in this function
    //let r0 = cursor.wrap1 * sx;
    //let rx = cursor.r - r0;
    let rx = cursor.x_hint;
    let wrap = cursor.wrap0 + 1;
    if wrap < cursor.wraps {
        Some(cursor_to_line_relative(text, sx, cursor, wrap, rx))
    } else {
        let line_inx = cursor.line_inx + 1;
        if line_inx < text.len_lines() - 1 {
            cursor_line_relative(text, sx, line_inx, 0, rx)
        } else {
            None
        }
    }
}

pub fn cursor_from_char(text: &Rope, sx: usize, c: usize, x_hint: usize) -> Cursor {
    //println!("cursor_from_char: {:?}", c);
    let line_inx = text.char_to_line(c);
    let lc0 = text.line_to_char(line_inx);
    let lc1 = text.line_to_char(line_inx+1);
    let line = text.line(line_inx).to_string();
    let elements = string_to_elements(&line);
    let wraps = elements.len().div_ceil(&sx);

    let r = Cursor::line_lc_to_r(&line, c - lc0);
    // current wrap
    let wrap0 = r / sx;

    Cursor {
        line_inx, x_hint: 0, c, r, wraps, wrap0, lc0, lc1,
        elements: elements.clone(),
        line
    }
}

pub fn string_to_elements(s: &String) -> Vec<ViewChar> {
    s.chars().fold(Vec::new(), |mut v , c| {
        match c {
            '\t' => {
                v.extend_from_slice(&[NOP, NOP, NOP, Tab]);
                v
            }
            '\n' => {
                v.push(NL);
                v
            }
            _ => {
                v.push(Char(c));
                v
            }
        }
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use ViewChar::*;

    #[test]
    fn test_cursor_next_visual_line() {
        let mut text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let mut c = cursor_start(&text, sx);
        let c_next = cursor_visual_next_line(&text, sx, &c);
        println!("r2:{:?}", (&c, &c_next));
    }

    #[test]
    fn test_cursor_visual() {
        let mut text = Rope::from_str("123456789\nabcdefghijk\na\nb\nc");
        let (sx, sy) = (5, 3);
        let c0 = cursor_from_char(&text, sx, 10, 0);
        println!("c0:{:?}", (&c0.to_string(sx)));
        let mut c = cursor_start(&text, sx);
        let mut i = 0;
        println!("c1:{:?}", (&c.to_string(sx)));
        loop {
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c1:{:?}", (i, &c.to_string(sx)));
                    i += 1;
                }
                None => break
            }
        }
        loop {
            match cursor_visual_prev_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c2:{:?}", (i, &c.to_string(sx)));
                    i += 1;
                }
                None => break
            }
        }
    }

    #[test]
    fn test_cursor_r_to_c() {
        let mut text = Rope::from_str("a\n12345\nc");
        let (sx, sy) = (3, 10);
        let mut cursor = cursor_start(&text, sx);
        for i in 0..20 {
            let r = cursor.lc_to_r(cursor.c - cursor.lc0);
            let c = cursor.r_to_c(cursor.r);
            println!("c:{:?}", (i, cursor.r, r, cursor.c, c));
            assert_eq!(cursor.c, c);
            assert_eq!(cursor.r, r);
            cursor = cursor_char_forward(&text, sx, &cursor, 1);
        }

    }

    #[test]
    fn test_cursor_backward() {
        let mut s = (0..10).map(|_| '\t').collect::<String>();
        s.push('\n');
        let mut text = Rope::from_str(&s);
        let (sx, sy) = (6, 10);
        let mut cursor = cursor_eof(&text, sx);
        for i in 0..20 {
            println!("c:{:?}", (i, cursor.r, cursor.c));
            cursor = cursor_char_backward(&text, sx, &cursor, 1);
        }

    }
}






