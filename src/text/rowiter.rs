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
                    //self.cursor.rx = 0;
                    //self.cursor.cx = 0;
                    let line = self.text.line(self.cursor.line_inx).to_string();
                    self.elements = string_to_elements(&line);
                    let row_iter = RowIter::new(self.text.clone(), self.sx, self.cursor.clone());
                    //info!("Line: {:?}", self.elements);
                    self.row_iter = row_iter;
                    self.row_iter.next(&self.elements)
                }
            }
        }
    }
    pub fn prev(&mut self) -> Option<RowItem> {
        //println!("pxx:{:?}", (self.cursor));
        match self.row_iter.prev(&self.elements) {
            Some(ri) => Some(ri),
            None => {
                if self.cursor.line_inx == 0 {
                    None
                } else {
                    // load the previous line
                    self.cursor.line_inx -= 1;
                    let line = self.text.line(self.cursor.line_inx).to_string();
                    self.elements = string_to_elements(&line);

                    //let wraps = self.elements.len().div_ceil(&self.sx);
                    //self.cursor.rx = (wraps - 1) * self.sx;
                    //self.cursor.cx = self.cursor.rx;
                    self.row_iter = RowIter::new(self.text.clone(), self.sx, self.cursor.clone());
                    //info!("line prev: {:?}", (self.cursor));
                    //self.row_iter.prev(&self.elements)
                    let mut it = RowIter::new(self.text.clone(), self.sx, self.cursor.clone());
                    it.next(&self.elements)
                    //Some(RowItem { Oelements, cursor: self.cursor.clone() });
                }
            }
        }
    }
}

impl RowIter {
    pub fn new(text: Rope, sx: usize, cursor: Cursor) -> Self {
        Self { text, sx, cursor }
    }
    pub fn next(&mut self, elements: &Vec<ViewChar>) -> Option<RowItem> {
        if elements.len() == 0 {
            return None;
        }
        let wraps = elements.len().div_ceil(&self.sx);
        let current = self.cursor.r / self.sx;
        //info!("rn: {:?}", (wraps, current));

        // check if current wrap has gone too far
        if current >= wraps {
            return None
        }

        // get current row
        let rx0 = current * self.sx;
        let start = rx0;
        let end = std::cmp::min(elements.len(), start + self.sx);
        let elements = elements[start..end].to_vec();
        let result = Some(RowItem { elements, cursor: self.cursor.clone() });
        result
    }

    pub fn prev(&mut self, elements: &Vec<ViewChar>) -> Option<RowItem> {
        //println!("pxr:{:?}", (self.cursor));
        if elements.len() == 0 {
            return None;
        }
        //let wraps = elements.len().div_ceil(&self.sx);
        let mut current = self.cursor.r / self.sx;
        //info!("row prev: {:?}", current);
        if current == 0 {
            return None
        }

        current -= 1;
        let rx0 = current * self.sx;
        // TODO
        //let cx0 = rx0;
        //self.cursor.rx = rx0;
        //self.cursor.cx = cx0;
        let start = rx0;
        //let end = start + self.sx;
        let end = std::cmp::min(elements.len(), start+self.sx);
        let elements = elements[start..end].to_vec();
        Some(RowItem { elements, cursor: self.cursor.clone() })
           // line_inx: self.cursor.line_inx, rx0, cx0 })
    }
}

//#[cfg(test)]
mod tests {
    use super::*;
    use ViewChar::*;
    #[test]
    fn test_rowiter_1() {
        let mut text = Rope::from_str("1234");
        let (sx, sy) = (10, 10);
        let c = cursor_start(&text, sx);
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        let r1 = it.next();
        it = LineWorker::iter(text.clone(), sx, c.clone());
        let r2 = it.prev();
        assert!(r1.is_some());
        assert!(r2.is_none());
    }

    //#[test]
    fn test_rowiter_prev() {
        let text = Rope::from_str("1234\na\nb");
        let (sx, _) = (10, 10);
        let mut c = cursor_start(&text, sx);
        assert_eq!(c.line_inx, 0);
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        // get the current
        let r1 = it.next();
        assert!(r1.is_some());
        // move to next line
        let r1 = it.next();
        assert!(r1.is_some());
        c = r1.unwrap().cursor.clone();
        assert_eq!(c.line_inx, 1);
        println!("c:{:?}", (c));

        it = LineWorker::iter(text.clone(), sx, c.clone());
        let r2 = it.prev();
        println!("prev1:{:?}", (r2));
        assert!(r2.is_some());
        c = r2.unwrap().cursor.clone();
        println!("prev:{:?}", (c));

    }
}
