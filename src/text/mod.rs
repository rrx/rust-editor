use ropey::Rope;

mod layout;
mod bufferblock;
mod macros;
pub mod input;
mod format;
pub mod terminal;
mod scroll;
mod render;
mod wrap;
pub mod cursor;
mod bufferview;
mod viewspec;
mod app;
pub mod smart;
pub mod linewrap;
pub mod viewport;
mod viewrow;
pub mod rowiter;
pub mod bufferlist;
pub mod textbuffer;
pub mod buffer;
pub mod lineworker;
pub mod display;
pub mod search;
pub mod window;

pub use smart::*;
pub use bufferblock::*;
pub use macros::*;
pub use input::*;
pub use format::*;
pub use layout::*;
pub use terminal::*;
pub use window::*;
pub use search::*;
pub use display::*;
pub use bufferview::*;
pub use viewspec::*;
pub use viewrow::*;
pub use linewrap::*;
pub use viewport::*;
pub use rowiter::*;
pub use bufferlist::*;
pub use textbuffer::*;
pub use buffer::*;
pub use app::*;
pub use wrap::WrapValue;
pub use cursor::*;
pub use lineworker::*;
pub use crate::bindings::parser::{ModeState, Motion};

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Easy,
    Cli
}
impl Default for Mode {
    fn default() -> Self { Self::Normal }
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct Register(pub char);

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub enum CliType {
    Cmd,
    SearchForward,
    SearchBackward
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Command {
    Insert(char),
    Join,
    Motion(usize, Motion),
    Delete(usize, Motion),
    Yank(Register, Motion),  // register, Motion
    Paste(usize, Register, Motion), // register, Motion
    Search(String),
    SearchInc(String),  // search incomplete
    RemoveChar(i32),
    Mode(Mode),
    MacroStart(MacroId),
    //CliStart(CliType),
    CliInc(char),
    CliEdit(Vec<Command>),
    CliExec,
    CliCancel,
    MacroEnd,
    Quit,
    Stop,
    Save,
    Resume,
    SaveBuffer(String, Rope),
    Mouse(u16, u16),
    Scroll(i16),
    ScrollPage(i8),
    Line(i64),
    LineNav(i32),
    Resize(u16,u16),
    MoveCursorY(i32),
    MoveCursorX(i32),
    BufferNext,
    BufferPrev,
    Test,
    Refresh
}

use std::convert::{From, Into};
//impl Into<Vec<Command>> for Command {
    //fn into(self) -> Vec<Command> {
        //vec![self]
    //}
//}

impl From<Command> for Vec<Command> {
    fn from(c: Command) -> Vec<Command> {
        vec![c]
    }
}

