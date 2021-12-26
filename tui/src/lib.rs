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
use layout::*;
use bufferblock::*;
use search::*;
use bufferlist::*;

// export
pub mod cli;
pub use layout::layout_cli;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Char(char),
    NOP,
    Tab,
    OOB,
    NL,
}


