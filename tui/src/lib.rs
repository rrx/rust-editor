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
pub use layout::layout_cli;
pub use bufferblock::BufferBlock;
pub use editor::EditorConfig;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Char(char),
    NOP,
    Tab,
    OOB,
    NL,
}


