#![feature(iter_advance_by)]
mod layout;
mod editor;
mod terminal;
mod display;
mod cursor;
mod search;
mod lineworker;
pub mod format;
mod bufferblock;
mod bufferlist;
mod viewchar;
mod row;

use cursor::*;
use format::*;
use display::*;
use terminal::*;
use search::*;
use bufferlist::*;
use row::*;

// export
pub use layout::layout_cli;
pub use bufferblock::BufferBlock;
pub use editor::EditorConfig;
pub use format::*;
pub use viewchar::{string_to_elements, ViewChar, ViewCharCollection, LineFormatType, FormatItem, grapheme_to_format_item , LineFormat};

