use log::*;
use lru::LruCache;
use ropey::Rope;
use super::*;
use num::Integer;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Char(char),
    NOP,
    Tab,
    OOB,
    NL
}

type ViewCharSlice<'a> = &'a [ViewChar];
//#[derive(Debug, Deref)]
//struct ViewCharSlice<'a>(&'a [ViewChar]);

//impl<'a> ViewCharSlice<'a> {
//}

#[derive(Debug)]
pub struct Line {
    s: String,
    size: usize,
    line_inx: usize,
    lc0: usize,
    sx: u16,
    wraps: usize,
    elements: Vec<ViewChar>
}

impl Default for Line {
    fn default() -> Self {
        Self::new(0, "".into(), 0, 0)
    }
}
impl Line {
    fn new(line_inx: usize, s: String, sx: u16, lc0: usize) -> Self {
        let mut out = Self {
            line_inx,
            s: "".into(),
            sx, elements: Vec::new(), wraps: 0, size: 0,
            lc0
        };
        out.update(s);
        out
    }

    fn update(&mut self, s: String) {
        let vsx = self.sx as usize;
        self.s = s;
        use ViewChar::*;
        self.elements = self.s.chars().fold(Vec::new(), |mut v , c| {
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
        });
        self.size = self.elements.iter().filter(|&c| c != &NOP).count();
        self.wraps = self.elements.len().div_ceil(&(self.sx as usize));
    }

    fn iter_at_char(&self, c: usize) -> RowIterator {
        // which wrap? is c in?
        // minimum is c-lc0, but depending on how many tabs there are
        // it will increase from there
        // effective position in the line = c - lc0 + 4*number_of_tabs before c
        // so we just count the number of tabs before c
        //
        use ViewChar::*;
        info!("I:{:?}", (c, self.lc0, &self.elements));
        let mut c0 = c;
        if c0 < self.lc0 {
            c0 = self.lc0;
        }
        let effective_x = self.elements.as_slice()[..c0-self.lc0].iter().filter(|&c| c != &Tab).count();
        let current_wrap = effective_x.div_ceil(&(self.sx as usize));
        RowIterator::new(&self.elements, self.sx as usize, current_wrap)
    }

    fn iter_at_wrap(&self, current: usize) -> RowIterator {
        RowIterator::new(&self.elements, self.sx as usize, current)
    }

    fn iter(&self) -> RowIterator {
        RowIterator::new(&self.elements, self.sx as usize, 0)
    }
}

struct RowIterator<'a> {
    elements: ViewCharSlice<'a>,
    sx: usize,
    current: usize
}
impl<'a> RowIterator<'a> {
    fn new(elements: ViewCharSlice<'a>, sx: usize, current: usize) -> Self {
        Self {
            elements, sx, current
        }
    }
}
impl<'a> Iterator for RowIterator<'a> {
    type Item = ViewCharSlice<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.elements.len() == 0 {
            return None;
        }
        let wraps = self.elements.len().div_ceil(&self.sx);
        if self.current >= wraps {
            return None;
        }
        let start = self.current * self.sx;
        let end = start + self.sx;
        self.current += 1;
        Some(&self.elements[start..std::cmp::min(self.elements.len(), end)])
    }
}

#[derive(Debug)]
struct Row {
    elements: Vec<ViewChar>,
    line_offset: usize
}

#[derive(Debug)]
pub struct LineWrap<'a> {
    dummy: &'a str,
    lines: LruCache<usize,Line>,
    rows: Vec<Row>,
    sx: u16,
    sy: u16,
    //_port: ViewPort
}

impl<'a> Default for LineWrap<'a> {
    fn default() -> Self {
        Self {
            dummy: "",
            lines: LruCache::new(100),
            rows: Vec::new(),
            //_port: ViewPort::default(),
            sx: 0,
            sy: 0
        }
    }
}

#[derive(Debug)]
pub struct Info {
    pub e: ViewChar,
    pub x: usize
}

impl<'a> LineWrap<'a> {
    //pub fn port(&self) -> &ViewPort {
        //&self._port
    //}

    pub fn update_spec(&mut self, sx: u16, sy: u16) {
        self.sx = sx;
        self.sy = sy;
    }
    //pub fn update_port(&mut self, port: ViewPort) {
        //self._port = port;
    //}

    fn update_line(&mut self, line_inx: usize) {
        if let None = self.lines.get(&line_inx) {
            let line = Line::default();
            self.lines.put(line_inx, line);
        }
    }

    pub fn get(&self, cx: u16, cy: u16) -> Info {
        info!("X:{:?}", (cx, cy, self.rows.len(), self.sx, self.sy));
        use ViewChar::*;
        let e = Info { e: OOB, x: 0 };
        if cx > self.sx {
            return e;
        }
        if cy as usize >= self.rows.len() {
            return e;
        }

        let row = self.rows.get(cy as usize).unwrap();
        if cx as usize >= row.elements.len() {
            return e;
        }
        let x = row.elements[..cx as usize].iter().filter(|&e| e != &NOP).count();
        Info { e: row.elements[cx as usize].clone(), x }
    }

    pub fn update_lines(&mut self, text: &Rope, port: &ViewPort) {
        let len_lines = text.len_lines();
        let len_chars = text.len_chars();
        let mut count = 0;
        let mut out = Vec::new();
        let mut c = port.char_start;


        while count < self.sy && c < len_chars {
            let line_inx = text.char_to_line(c);
            let lc0 = text.line_to_char(line_inx);
            let s = text.line(line_inx).to_string();
            info!("X1:{:?}", (line_inx, c, lc0));
            let line = Line::new(line_inx, s, self.sx, lc0);
            let mut iter = line.iter_at_char(c);

            //let s = text.line(line_inx).to_string();
            //let line = Line::new(line_inx, s, self.sx, lc0);
            //let mut iter = line.iter();
            let mut wc = 0;
            let mut line_offset = 0;
            while count < self.sy {
                match iter.next() {
                    Some(x) => {
                        info!("X:{:?}", (line_inx, count, wc, line_offset, x));
                        let elements = x.iter().cloned().collect::<Vec<_>>();
                        let row_size = elements.iter().filter(|&c| c != &ViewChar::NOP).count();
                        out.push(Row { elements, line_offset });
                        count += 1;
                        line_offset += row_size;
                    }
                    None => break
                }
                wc += 1;
            }
            c = text.line_to_char(line_inx+1);
            //line_inx += 1;
        }
        self.rows = out;
    }
}

