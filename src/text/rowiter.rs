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
                    //info!("Line: {:?}", self.elements);
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

                    let wraps = self.elements.len().div_ceil(&self.sx);
                    self.cursor.rx = (wraps - 1) * self.sx;
                    self.cursor.cx = self.cursor.rx;
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

pub struct LineWorker { }
impl LineWorker {
    pub fn render(text: Rope, spec: &ViewSpec, start: Cursor, cursor: Cursor) -> (Cursor, Vec<DrawCommand>) {
        let sx = spec.sx as usize;
        let sy = spec.sy as usize;
        let header = spec.header as usize;

        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), cursor.clone());
        //info!("rows: {:?}", rows);
        let start = rows[0].cursor.clone();

        let mut out = Vec::new();
        if spec.header > 0 {
            out.push(DrawCommand::Status(out.len() as u16, format!("Header: {:?}", cursor).into()));
        }

        let row_inx = out.len() as u16;
        rows.iter().enumerate().map(|(inx, row)| {
            let mut line_inx = 0;
            if row.cursor.rx < sx {
                line_inx = row.cursor.line_inx + 1;
            }
            DrawCommand::Line(row_inx + inx as u16, line_inx, row.to_string())
        }).for_each(|c| {
            out.push(c);
        });

        while out.len() < sy + header {
            out.push(DrawCommand::Row(0, out.len() as u16, ";".into()));
        }

        if spec.status > 0 {
            out.push(DrawCommand::Status(out.len() as u16, format!("DEBUG: {:?}", cursor).into()));
        }

        if spec.footer > 0 {
            let start = rows[0].cursor.clone();
            out.push(DrawCommand::Status(out.len() as u16, format!("[{},{}] S: {:?}", cx, cy, &start).into()));
        }

        out.push(DrawCommand::Cursor(cx + spec.x0, cy + spec.y0));
        (start, out)
    }

    pub fn cursor_last_line(text: Rope, sx: usize) -> Cursor {
        let mut cursor = Cursor::default();
        cursor.line_inx = text.len_lines() - 1;
        cursor
    }

    pub fn screen(text: Rope, sx: usize, sy: usize, start: Cursor, cursor: Cursor) -> (u16, u16, Vec<RowItem>) {
        // start with the current position, iterate back until we find the start, or we fill up the
        // screen
        // iterate next until we fill up the screen
        //let mut count = 0;

        let mut c = cursor.clone();

        //info!("screen: {:?}", (&c, &text.len_lines()));
        let line_max = text.len_lines() - 1;

        if c.line_inx >= line_max {
            c.line_inx = line_max - 1;
            c.rx = 0;
            c.cx = 0;
        }

        let mut p_iter = Self::iter(text.clone(), sx, c.clone());
        let mut n_iter = Self::iter(text.clone(), sx, c.clone());
        let mut out = Vec::new();
        let mut cx = 0;
        let mut cy = 0;

        // current line should always succeed
        let current = n_iter.next().unwrap();

        cx = current.cursor.rx % sx;
        out.push(current);

        let mut count = 0;
        while let Some(row) = p_iter.prev() {
            info!("p1: {:?}", (&row.cursor, &start, row.cursor == start));
            if row.cursor.line_inx < start.line_inx {
                break;
            }
            if row.cursor.line_inx == start.line_inx {
                let wraps0 = row.cursor.rx / sx;
                let wraps1 = start.rx / sx;
                if wraps0 < wraps1 {
                    break;
                }
            }

            if out.len() >= sy {
                break;
            }
            //if row.cursor > cursor || out.len() >= sy {
                //break;
            //}
            out.insert(0, row);
            cy += 1;
            count += 1;
        }
        info!("px: {:?}", (sy, cy, out.len(), count));//&row.cursor, &start, row.cursor == start));


        while out.len() < sy {
            if let Some(row) = n_iter.next() {
                out.push(row);
            } else {
                break;
            }
        }
        (cx as u16, cy, out)
    }

    pub fn current(text: Rope, sx: usize, cursor: Cursor) -> RowItem {
        let mut iter = Self::iter(text.clone(), sx, cursor.clone());
        iter.next().unwrap()
    }

    pub fn move_y(text: Rope, sx: usize, cursor: Cursor, dy: i32) -> Cursor {
        let mut iter = Self::iter(text.clone(), sx, cursor.clone());
        let mut c = cursor.clone();
        if dy > 0 {
            let mut count = 0;
            iter.next();
            loop {
                if count >= dy {
                    break;
                }
                match iter.next() {
                    Some(row) => {
                        c = row.cursor;
                        count += 1;
                    }
                    None => break
                }
            }
        } else if dy < 0 {
            let mut count = 0;
            loop {
                if count <= dy {
                    break;
                }
                match iter.prev() {
                    Some(row) => {
                        c = row.cursor;
                        count -= 1;
                    },
                    None => break
                }
            }
        }
        c
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

        // increment iterator
        current += 1;
        self.cursor.rx = current * self.sx;
        // TODO
        self.cursor.cx = self.cursor.rx;
        result
    }

    pub fn prev(&mut self, elements: &Vec<ViewChar>) -> Option<RowItem> {
        //println!("pxr:{:?}", (self.cursor));
        if elements.len() == 0 {
            return None;
        }
        let wraps = elements.len().div_ceil(&self.sx);
        let mut current = self.cursor.rx / self.sx;
        //info!("row prev: {:?}", current);
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
        //let end = start + self.sx;
        let end = std::cmp::min(elements.len(), start+self.sx);
        let elements = elements[start..end].to_vec();
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
    fn test_rowiter_prev() {
        let mut c = Cursor::default();
        //println!("prev:{:?}", (c));
        let mut text = Rope::from_str("1234\na\nb");
        let (sx, sy) = (10, 10);
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
    #[test]
    fn test_rowiter_2() {
        let c = Cursor::default();
        let start = Cursor::default();
        let mut text = Rope::from_str("123456789\nabcdefghijk\n");
        let (sx, sy) = (5, 2);
        let mut it = LineWorker::iter(text.clone(), sx, c.clone());
        println!("2:{:?}", (text.len_lines()));
        while let Some(x) = it.next() {
            println!("next: {:?}", x.to_string());
        }

        let (cx, cy, rows) = LineWorker::screen(text, sx, sy, start, c);
        assert_eq!(sy, rows.len());
        rows.iter().for_each(|row| {
            println!("R: {:?}", (row.to_string(), row));
        });
    }

    #[test]
    fn test_rowiter_move_y() {
        let mut c = Cursor::default();
        let mut start = Cursor::default();
        let mut text = Rope::from_str("123456789\nabcdefghijk\na\nb\nc");
        let (sx, sy) = (5, 3);
        for i in 0..8 {
            let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
            start = rows[0].cursor.clone();
            println!("current:{:?}", (i, cx, cy, &start, &c));
            rows.iter().enumerate().for_each(|(i2, row)| {
                let x;
                if cy == (i2 as u16) {
                    x = '*';
                } else {
                    x = ' ';
                }
                println!("\t{}r:{:?}", x, (i2, row.to_string()));
            });
            //let current = LineWorker::current(text.clone(), sx, c.clone());
            //println!("current:{:?}", (i, current));
            c = LineWorker::move_y(text.clone(), sx, c.clone(), 1);
        }
        for i in 0..8 {
            let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
            start = rows[0].cursor.clone();
            println!("current:{:?}", (i, cx, cy, &start, &c));
            rows.iter().enumerate().for_each(|(i2, row)| {
                let x;
                if cy == (i2 as u16) {
                    x = '*';
                } else {
                    x = ' ';
                }
                println!("\t{}r:{:?}", x, (i2, row.to_string()));
            });
            c = LineWorker::move_y(text.clone(), sx, c.clone(), -1);
            //println!("c:{:?}", (c));
        }
    }

    #[test]
    fn test_rowiter_move_y_2() {
        let mut c = Cursor::default();
        let mut start = Cursor::default();
        let mut text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);

        // init
        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
        start = rows[0].cursor.clone();
        println!("r0:{:?}", (&c, &start));


        c = LineWorker::move_y(text.clone(), sx, c.clone(), 1);
        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
        start = rows[0].cursor.clone();
        println!("r1:{:?}", (&c, &start));

        c = LineWorker::move_y(text.clone(), sx, c.clone(), -1);
        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));
    }

    #[test]
    fn test_rowiter_last_line() {
        let mut c = Cursor::default();
        let mut start = Cursor::default();
        let mut text = Rope::from_str("a\nb\nc");
        let (sx, sy) = (10, 10);
        let lines: usize = text.len_lines() - 1;
        c = LineWorker::cursor_last_line(text.clone(), sx);
        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));

        c = LineWorker::cursor_last_line(text.clone(), sx);
        c.line_inx = 100;
        let (cx, cy, rows) = LineWorker::screen(text.clone(), sx, sy, start.clone(), c.clone());
        start = rows[0].cursor.clone();
        println!("r2:{:?}", (&c, &start));
    }

}




