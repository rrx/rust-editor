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
    pub r0: usize, // render index for start of wrap, relative to start of line
    pub r1: usize, // render index for end of wrap, relative to start of line
    pub c0: usize, // char for start of wrap relative to start of file
    pub c1: usize, // char for end of wrap relative to start of file
    pub cx: usize, // char position relative to the start of wrap
    pub rx: usize, // rendered position from start of wrap
    pub line: String,
    pub elements: Vec<ViewChar> // cached line
    //pub elements: &'a [ViewChar]
}
impl Cursor {
    pub fn simple_format(&self) -> String {
        format!("(Line:{},rx:{},cx:{})", self.line_inx, self.rx, self.cx)
    }
    pub fn to_elements(&self) -> Vec<ViewChar> {
        self.elements.as_slice()[self.r0..self.r1].iter().cloned().collect()
    }
    pub fn to_string(&self) -> String {
        self.line.chars().skip(self.c0).take(self.c1-self.c0).collect()
    }
}
use std::cmp::{Ord, Ordering};
impl PartialEq for Cursor {
    fn eq(&self, other: &Self) -> bool {
        self.line_inx == other.line_inx && self.cx == other.cx
    }
}
impl Eq for Cursor {}
impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> Ordering {
        self.line_inx.cmp(&other.line_inx).then(self.cx.cmp(&other.cx))
    }
}
impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
//impl Default for Cursor {
    //fn default() -> Self {
        //Self { line_inx: 0, cx: 0, rx: 0, x_hint: 0, c: 0 }
    //}
//}

pub fn cursor_eof(text: &Rope, sx: usize) -> Cursor {
    cursor_from_char(text, sx, text.len_chars())
}

pub fn cursor_start(text: &Rope, sx: usize) -> Cursor {
    cursor_from_char(text, sx, 0)
}

pub fn cursor_from_line(text: &Rope, sx: usize, line_inx: usize) -> Cursor {
    let c = text.line_to_char(line_inx);
    cursor_from_char(text, sx, c)
}

pub fn cursor_to_row(cursor: &Cursor) -> RowItem {
    RowItem { elements: cursor.to_elements(), cursor: cursor.clone() }
}

pub fn cursor_to_line_x(text: &Rope, sx: usize, cursor: &Cursor, x: i32) -> Cursor {
    let mut line_x: usize = x as usize;
    if x < 0 {
        line_x = cursor.elements.len() - i32::abs(x) as usize;
    }
    if line_x > (cursor.elements.len() - 1) {
        line_x = cursor.elements.len() - 1;
    }
    let wrap0 = line_x / sx;
    let rx = line_x % sx;
    info!("cursor_to_line_x: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), x, wrap0, rx));
    cursor_to_line_relative(text, sx, cursor, wrap0, rx)
}

pub fn cursor_to_relative_x(text: &Rope, sx: usize, cursor: &Cursor, dx: i32) -> Cursor {
    info!("cursor_to_relative_x: {:?}", (cursor.line_inx, cursor.r, cursor.elements.len(), dx));
    if dx < 0 {
        let dx_back = i32::abs(dx) as usize;
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
                cursor_to_relative_x(text, sx, &prev2, -1 * remainder as i32)
            } else {
                cursor_to_line_x(text, sx, &cursor, 0) // goto the start of the file
            }
        }
    } else if dx > 0 {
        let dx_forward = dx as usize;
        let remainder = cursor.elements.len() - cursor.r;
        if remainder <= dx_forward {
            let line_inx = cursor.line_inx + 1;
            if line_inx >= text.len_lines() - 1 {
                cursor_to_line_x(text, sx, cursor, -1) // go to the end of the line if this is the last line
            } else {
                let next = cursor_from_line(text, sx, cursor.line_inx + 1);
                cursor_to_relative_x(text, sx, &next, (dx_forward - remainder) as i32)
            }
        } else {
            let x = cursor.r + dx_forward;
            cursor_to_line_x(text, sx, cursor, x as i32)
        }
    } else {
        cursor.clone()
    }
}

pub fn cursor_to_line_relative(text: &Rope, sx: usize, cursor: &Cursor, wrap: usize, rx: usize) -> Cursor {
    info!("cursor_to_line_relative: {:?}", (cursor.line_inx, wrap, rx));
    let mut c = cursor.clone();
    let r = std::cmp::min(c.elements.len() - 1, wrap * sx + rx);
    c.wrap0 = r / sx;
    c.r = r;
    c.c = c.lc0 + c.elements.as_slice()[..c.r].iter().filter(|&c| c != &NOP).count();

    // render index for start and end of word wrapped line, in rendered elements
    c.r0 = c.wrap0 * sx;
    c.r1 = std::cmp::min(c.elements.len(), (c.wrap0+1) * sx);
    c.rx = c.r - c.r0;

    c.c0 = c.lc0 + c.elements.as_slice()[..c.r0].iter().filter(|&c| c != &NOP).count();
    c.c1 = c.c0 + c.elements.as_slice()[c.r0..c.r1].iter().filter(|&c| c != &NOP).count();

    c.cx = c.c - c.c0;
    //println!("to_line_relative1:{:?}", (sx, wrap, rx, c.to_string()));
    //println!("to_line_relative2:{:?}", (&cursor));
    //println!("to_line_relative3:{:?}", (&c));
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
    if cursor.wrap0 > 0 {
        //println!("cursor_visual_prev_line:{:?}", (cursor.line_inx, cursor.wrap0, cursor.rx));
        Some(cursor_to_line_relative(text, sx, &cursor, cursor.wrap0 - 1, cursor.rx))
    } else {
        if cursor.line_inx == 0 {
            return None;
        } else {
            let c = cursor_from_line(&text, sx, cursor.line_inx - 1);
            Some(cursor_to_line_relative(text, sx, &c, c.wraps - 1, cursor.rx))
        }
    }
}

pub fn cursor_visual_next_line(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    let wrap = cursor.wrap0 + 1;
    if wrap < cursor.wraps {
        Some(cursor_to_line_relative(text, sx, cursor, wrap, cursor.rx))
    } else {
        info!("cursor_visual_next_line:{:?}", (cursor.line_inx, text.len_lines()));
        let line_inx = cursor.line_inx + 1;
        if line_inx < text.len_lines() - 1 {
            cursor_line_relative(text, sx, line_inx, 0, cursor.rx)
        } else {
            None
        }
    }
}

pub fn cursor_visual_next_line2(text: &Rope, sx: usize, cursor: &Cursor) -> Option<Cursor> {
    // take c, add to c until r in the right spot
    let mut c = cursor.c;
    let mut r = cursor.r;
    let rx = cursor.rx;
    let r_end = std::cmp::min(cursor.elements.len(), cursor.r + sx);
    let mut it = text.chars_at(cursor.c);
    while let Some(ch) = it.next() {
        c+=1;
        if ch == '\n' {
            r += sx - rx;
        } else if ch == '\t' {
            r += 4;
        } else {
            r += 1;
        }
        if r >= r_end {
            return Some(cursor_from_char(text, sx, c));
        }
    }
    None
}


pub fn cursor_from_char(text: &Rope, sx: usize, c: usize) -> Cursor {
    //println!("cursor_from_char: {:?}", c);
    let line_inx = text.char_to_line(c);
    let lc0 = text.line_to_char(line_inx);
    let lc1 = text.line_to_char(line_inx+1);
    let line = text.line(line_inx).to_string();
    let elements = string_to_elements(&line);
    let wraps = elements.len().div_ceil(&sx);

    let number_of_tabs = line.chars().take(c-lc0).filter(|&c| c == '\t').count();
    let r = c - lc0 + 4*number_of_tabs;

    // current wrap
    let wrap0 = r / sx;

    // render index for start and end of word wrapped line, in rendered elements
    let r0 = wrap0 * sx;
    let r1 = std::cmp::min(elements.len(), (wrap0+1) * sx);
    let rx = r - r0;

    let c0 = lc0 + elements.as_slice()[..r0].iter().filter(|&c| c != &NOP).count();
    let c1 = c0 + elements.as_slice()[r0..r1].iter().filter(|&c| c != &NOP).count();

    let cx = c - c0;

    Cursor {
        line_inx, rx, cx, x_hint: 0, c, r, wraps, wrap0, lc0, lc1, c0, c1, r0, r1,
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
    fn test_rowiter_next_visual_line() {
        let mut text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let mut c = cursor_start(&text, sx);
        let c_next = cursor_visual_next_line(&text, sx, &c);
        println!("r2:{:?}", (&c, &c_next));
    }

    #[test]
    fn test_rowiter_cursor_visual() {
        let mut text = Rope::from_str("123456789\nabcdefghijk\na\nb\nc");
        let (sx, sy) = (5, 3);
        let c0 = cursor_from_char(&text, sx, 10);
        println!("c0:{:?}", (&c0.to_string()));
        let mut c = cursor_start(&text, sx);
        let mut i = 0;
        println!("c1:{:?}", (&c.to_string()));
        loop {
            match cursor_visual_next_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c1:{:?}", (i, &c.to_string()));
                    i += 1;
                }
                None => break
            }
        }
        loop {
            match cursor_visual_prev_line(&text, sx, &c) {
                Some(x) => {
                    c = x;
                    println!("c2:{:?}", (i, &c.to_string()));
                    i += 1;
                }
                None => break
            }
        }
    }

}





