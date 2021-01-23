use log::*;
use ropey::Rope;
use super::*;
use num::Integer;
use crate::text::cursor::Cursor;

#[derive(Debug)]
pub struct RowIter {
    text: Rope,
    cursor: Cursor,
    sx: usize
}

#[derive(Debug)]
pub struct RowItem {
    pub elements: Vec<ViewChar>,
    pub cursor: Cursor
    //line_inx: usize,
    //rx0: usize,
    //cx0: usize
}
impl RowItem {
    pub fn to_string(&self) -> String {
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

use ViewChar::*;
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

pub struct LineIter {
    text: Rope,
    cursor: Cursor,
    sx: usize,
    row_iter: RowIter,
    elements: Vec<ViewChar>
}

impl LineIter {
    pub fn new(text: Rope, sx: usize, cursor: Cursor) -> Self {
        let line = text.line(cursor.line_inx).to_string();
        let row_iter = RowIter::new(text.clone(), sx, cursor.clone());
        Self {
            text, sx, cursor, row_iter,
            elements: string_to_elements(&line)
        }
    }

    pub fn next(&mut self) -> Option<RowItem> {
        match self.row_iter.next(&self.elements) {
            Some(ri) => Some(ri),
            None => {
                self.cursor.line_inx += 1;
                if self.cursor.line_inx >= self.text.len_lines() {
                    None
                } else {
                    self.cursor.rx = 0;
                    self.cursor.cx = 0;
                    let line = self.text.line(self.cursor.line_inx).to_string();
                    self.elements = string_to_elements(&line);
                    self.row_iter = RowIter::new(self.text.clone(), self.sx, self.cursor.clone());
                    info!("Line: {:?}", self.elements);
                    self.row_iter.next(&self.elements)
                }
            }
        }
    }
    pub fn prev(&mut self) -> Option<RowItem> {
        match self.row_iter.prev(&self.elements) {
            Some(ri) => Some(ri),
            None => {
                if self.cursor.line_inx == 0 {
                    None
                } else {
                    self.cursor.line_inx -= 1;
                    let wraps = self.elements.len().div_ceil(&self.sx);
                    self.cursor.rx = (wraps - 1) * self.sx;
                    self.cursor.cx = self.cursor.rx;
                    let line = self.text.line(self.cursor.line_inx).to_string();
                    self.elements = string_to_elements(&line);
                    self.row_iter = RowIter::new(self.text.clone(), self.sx, self.cursor.clone());
                    self.row_iter.prev(&self.elements)
                }
            }
        }
    }
}

pub struct LineWorker { }
impl LineWorker {
    pub fn screen(text: Rope, sx: usize, sy: usize, cursor: Cursor) -> Vec<RowItem> {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        //let mut count = 0;
        let mut p_iter = Self::iter(text.clone(), sx, cursor.clone());
        let mut n_iter = Self::iter(text.clone(), sx, cursor.clone());
        let mut out = Vec::new();
        //
        // current line should always succeed
        out.push(n_iter.next().unwrap());

        while let Some(row) = p_iter.prev() {
            if row.cursor > cursor || out.len() >= sy {
                break;
            }
            out.push(row);
        }

        while out.len() < sy {
            if let Some(row) = n_iter.next() {
                out.push(row);
            } else {
                break;
            }
        }
        out
    }

    pub fn iter(text: Rope, sx: usize, cursor: Cursor) -> LineIter {
        LineIter::new(text, sx, cursor)
    }
}

//self.size = self.elements.iter().filter(|&c| c != &NOP).count();
//self.wraps = self.elements.len().div_ceil(&(self.sx as usize));

impl RowIter {
    pub fn new(text: Rope, sx: usize, cursor: Cursor) -> Self {
        Self { text, sx, cursor }
    }
    pub fn next(&mut self, elements: &Vec<ViewChar>) -> Option<RowItem> {
        if elements.len() == 0 {
            return None;
        }
        let wraps = elements.len().div_ceil(&self.sx);
        let mut current = self.cursor.rx / self.sx;
        info!("rn: {:?}", (wraps, current));

        // check if current wrap has gone too far
        if current >= wraps {
            return None
        }

        // get current row
        let rx0 = current * self.sx;
        let start = rx0;
        let end = start + self.sx;
        let elements = elements[start..std::cmp::min(elements.len(), end)].to_vec();
        let result = Some(RowItem { elements, cursor: self.cursor.clone() });
            //line_inx: self.cursor.line_inx, rx0, cx0: rx0 });

        // increment iterator
        current += 1;
        self.cursor.rx = current * self.sx;
        // TODO
        self.cursor.cx = self.cursor.rx;
        result
    }

    pub fn prev(&mut self, elements: &Vec<ViewChar>) -> Option<RowItem> {
        if elements.len() == 0 {
            return None;
        }
        //let wraps = elements.len().div_ceil(&self.sx);
        let mut current = self.cursor.rx / self.sx;
        if current == 0 {
            return None
        }

        current -= 1;
        let rx0 = current * self.sx;
        // TODO
        let cx0 = rx0;
        self.cursor.rx = rx0;
        self.cursor.cx = cx0;
        let start = rx0;
        let end = start + self.sx;
        let elements = elements[start..std::cmp::min(elements.len(), end)].to_vec();
        Some(RowItem { elements, cursor: self.cursor.clone() })
           // line_inx: self.cursor.line_inx, rx0, cx0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ViewChar::*;
    #[test]
    fn test_rowiter_1() {
        let c = Cursor::default();
        let mut text = Rope::from_str("1234");
        let (sx, sy) = (10, 10);
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        let r1 = it.next();
        it = LineWorker::iter(text.clone(), sx, c.clone());
        let r2 = it.prev();
        assert!(r1.is_some());
        assert!(r2.is_none());
    }
    #[test]
    fn test_rowiter_2() {
        let c = Cursor::default();
        let mut text = Rope::from_str("123456789\nabcdefghijk\n");
        let (sx, sy) = (5, 2);
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        println!("2:{:?}", (text.len_lines()));
        while let Some(x) = it.next() {
            println!("next: {:?}", x.to_string());
        }

        let s = LineWorker::screen(text, sx, sy, c);
        assert_eq!(sy, s.len());
        s.iter().for_each(|row| {
            println!("R: {:?}", (row.to_string(), row));
        });

        //let r1 = it.next();
        //it = w.iter(c.clone());
        //let r2 = it.prev();
        //assert!(r1.is_none());
        //assert!(r2.is_none());
    }
}




