use ropey::Rope;

pub mod buffer;
mod bufferblock;
pub mod bufferlist;
pub mod cursor;
pub mod display;
mod format;
pub mod input;
mod layout;
pub mod lineworker;
mod macros;
pub mod search;
pub mod terminal;
pub mod window;

pub use crate::bindings::parser::{ModeState, Motion};
pub use buffer::*;
pub use bufferblock::*;
pub use bufferlist::*;
pub use cursor::*;
pub use display::*;
pub use format::*;
pub use input::*;
pub use layout::*;
pub use lineworker::*;
pub use macros::*;
pub use search::*;
pub use terminal::*;
pub use window::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ViewChar {
    Char(char),
    NOP,
    Tab,
    OOB,
    NL,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Easy,
    Cli,
}
impl Default for Mode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Copy, Clone)]
pub struct Register(pub char);

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Command {
    Insert(char),
    Join,
    Motion(usize, Motion),
    Delete(usize, Motion),
    Yank(Register, Motion),         // register, Motion
    Paste(usize, Register, Motion), // repetitions, register, Motion
    RemoveChar(i32),
    Mode(Mode),
    MacroStart(MacroId),
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
    Resize(u16, u16),
    MoveCursorY(i32),
    MoveCursorX(i32),
    BufferNext,
    BufferPrev,
    Test,
    Refresh,
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
