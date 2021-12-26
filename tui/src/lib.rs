#![feature(iter_advance_by)]
mod layout;
mod editor;
mod terminal;
mod display;
mod cursor;
mod search;
mod lineworker;
mod format;
mod bufferblock;
mod bufferlist;

use cursor::*;
use format::*;
use display::*;
use terminal::*;
use search::*;
use bufferlist::*;

// export
pub use layout::layout_cli;
pub use bufferblock::BufferBlock;
pub use editor::EditorConfig;

use editor_core::{grapheme_width};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Grapheme(String),
    //NOP,
    Tab(u8),
    Control(u8),
    //OOB,
    NL,
}

impl ViewChar {
    pub fn unicode_width(&self) -> usize {
        match self {
            ViewChar::Grapheme(s) => grapheme_width(&s),
            ViewChar::Tab(size) => *size as usize,
            ViewChar::NL => 1,
            ViewChar::Control(v) => 3
        }
    }
    pub fn char_length(&self) -> usize {
        match self {
            ViewChar::Grapheme(s) => s.len(),
            ViewChar::Tab(size) => *size as usize,
            ViewChar::NL => 1,
            ViewChar::Control(v) => 3
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ViewCharWithSize(ViewChar, usize);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ViewCharCollection {
    elements: Vec<ViewCharWithSize>
}

impl Default for ViewCharCollection {
    fn default() -> Self {
        ViewCharCollection { elements: vec![] }
    }
}

fn add_size(v: ViewChar) -> ViewCharWithSize {
    let width = v.unicode_width();
    ViewCharWithSize(v, width)
}

impl ViewCharCollection {
    pub fn unicode_width(&self) -> usize {
        self.unicode_width_range(0,self.elements.len())
    }

    pub fn unicode_width_range(&self, start: usize, end: usize) -> usize {
        self.elements.as_slice()[start..end].iter().fold(0, |acc, e| acc + e.0.unicode_width())
    }

    pub fn char_length(&self) -> usize {
        self.char_length_range(0, self.elements.len())
    }

    pub fn char_length_range(&self, start: usize, end: usize) -> usize {
        let mut a = start;
        if a >= self.elements.len() {
            a = self.elements.len();
        }
        let mut b = end;
        if b > self.elements.len() {
            b = self.elements.len();
        }

        self.elements.as_slice()[a..b].iter().fold(0, |acc, e| acc + e.0.char_length())
    }

    pub fn append(&mut self, v: &mut Vec<ViewChar>) {
        self.elements.append(&mut v.iter().map(|x| add_size(x.clone())).collect::<Vec<ViewCharWithSize>>());
    }

    pub fn push(&mut self, v: ViewChar) {
        self.elements.push(add_size(v));
    }
}



