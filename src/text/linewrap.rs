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
    //translated: String,
    size: usize,
    line_inx: usize,
    sx: u16,
    wraps: usize,
    elements: Vec<ViewChar>
}

impl Default for Line {
    fn default() -> Self {
        Self::new(0, "".into(), 0)
    }
}
impl Line {
    fn new(line_inx: usize, s: String, sx: u16) -> Self {
        let mut out = Self {
            line_inx,
            s: "".into(),
            sx, elements: Vec::new(), wraps: 0, size: 0
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
        self.wraps = self.elements.len() / vsx + 1;
    }

    fn iter(&self) -> RowIterator {
        RowIterator::new(&self.elements, self.sx as usize)
    }
}

struct RowIterator<'a> {
    elements: ViewCharSlice<'a>,
    sx: usize,
    current: usize
}
impl<'a> RowIterator<'a> {
    fn new(elements: ViewCharSlice<'a>, sx: usize) -> Self {
        Self {
            elements, sx, current: 0
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


//impl Default for Row {
    //fn default() -> Self {
        //Self { s: "".into() }
    //}
//}

#[derive(Debug)]
pub struct LineWrap<'a> {
    dummy: &'a str,
    lines: LruCache<usize,Line>,
    rows: Vec<Row>,
    sx: u16,
    sy: u16,
    _port: ViewPort
}

impl<'a> Default for LineWrap<'a> {
    fn default() -> Self {
        Self {
            dummy: "",
            lines: LruCache::new(100),
            rows: Vec::new(),
            _port: ViewPort::default(),
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
    pub fn port(&self) -> &ViewPort {
        &self._port
    }

    pub fn update_spec(&mut self, sx: u16, sy: u16) {
        self.sx = sx;
        self.sy = sy;
    }
    pub fn update_port(&mut self, port: ViewPort) {
        self._port = port;
    }

    fn update_line(&mut self, line_inx: usize) {
        if let None = self.lines.get(&line_inx) {
            let line = Line::default();
            self.lines.put(line_inx, line);
        }
    }

    pub fn get(&self, cx: u16, cy: u16) -> Info {
        info!("X:{:?}", (cx, cy, self.rows.len(), self.sx, self.sy));
        use ViewChar::*;
        let mut e = Info { e: OOB, x: 0 };
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

    pub fn update_lines(&mut self, text: &Rope) {
        let mut line_inx = text.char_to_line(self._port.char_start);
        let len_lines = text.len_lines();
        let mut count = 0;
        let mut out = Vec::new();
        while count < self.sy && line_inx < len_lines {
            let s = text.line(line_inx).to_string();
            let line = Line::new(line_inx, s, self.sx);
            let mut iter = line.iter();
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
            line_inx += 1;
        }
        self.rows = out;
    }
}

