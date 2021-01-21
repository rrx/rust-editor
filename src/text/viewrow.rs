use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use super::*;

#[derive(Debug)]
pub enum RowType {
    Line(String),
    Eof
}

#[derive(Debug)]
pub struct ViewRow {
    body: RowType,
    checksum: u64,
    line: usize,
    pub c0: usize,
    pub c1: usize,
    pub dirty: bool
}

impl ViewRow {
    pub fn new(body: String) -> Self {
        Self { body: RowType::Line(body), checksum: 0, dirty: false, line: 0, c0: 0, c1: 0 }.init()
    }

    fn init(mut self) -> Self {
        self.update_hash();
        self.dirty = true;
        self
    }

    pub fn is_within(&self, c: usize) -> bool {
        self.c0 == self.c1 || (self.c0 <= c && c < self.c1)
    }

    pub fn is_line(&self) -> bool {
        match self.body {
            RowType::Line(_) => true,
            _ => false
        }
    }

    pub fn to_string(&self) -> String {
        let s = match &self.body {
            RowType::Line(x) => String::from(x),
            _ => "".into()
        }.replace("\n", ".");
        s
    }

    fn update_hash(&mut self) -> bool {
        let mut h = DefaultHasher::new();
        let v = match &self.body {
            RowType::Line(s) => {
                s.hash(&mut h);
                h.finish()
            }
            RowType::Eof => 0
        };
        let changed = v != self.checksum;
        self.checksum = v;
        changed
    }

    pub fn make_eof(&mut self) {
        self.body = RowType::Eof;
        self.dirty = self.update_hash();
    }

    pub fn update_wrap(&mut self, w: &Wrap) {
        self.line = w.line0;
        self.c0 = w.c0;
        self.c1 = w.c1;
    }

    pub fn update_string(&mut self, body: String) {
        self.body = RowType::Line(body);
        self.dirty = self.update_hash();
    }

    pub fn clear(&mut self) {
        self.dirty = false;
    }
}

impl Default for ViewRow {
    fn default() -> Self {
        Self::new("".into())
    }
}

use std::cmp::{Eq, PartialEq};
impl PartialEq for ViewRow {
    fn eq(&self, other: &Self) -> bool {
        self.checksum == other.checksum
    }
}
impl Eq for ViewRow {}


