#![feature(iter_advance_by)]
mod bufferblock;
mod bufferlist;
mod cursor;
mod display;
mod editor;
pub mod format;
mod layout;
mod lineworker;
mod row;
mod search;
mod terminal;
mod viewchar;

use bufferlist::*;
use cursor::*;
use display::*;
use format::*;
use row::*;
use search::*;
use terminal::*;

// export
pub use bufferblock::BufferBlock;
pub use editor::{Editor, EditorComplexLayout, EditorConfig, EditorSimpleLayout};
pub use format::*;
pub use layout::event_loop;
pub use viewchar::{
    grapheme_to_format_item, string_to_elements, FormatItem, LineFormat, LineFormatType, ViewChar,
    ViewCharCollection,
};
