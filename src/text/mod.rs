use ropey::Rope;

mod bufferblock;
pub mod bufferlist;
pub mod config;
pub mod cursor;
pub mod display;
pub mod editor;
mod format;
pub mod input;
mod layout;
pub mod lineworker;
mod macros;
pub mod registers;
pub mod search;
pub mod terminal;

pub use self::editor::*;
pub use crate::bindings::parser::{ModeState, Motion};
pub use bufferblock::*;
pub use bufferlist::*;
pub use config::*;
pub use cursor::*;
pub use display::*;
pub use format::*;
pub use input::*;
pub use layout::*;
pub use lineworker::*;
pub use macros::*;
pub use registers::*;
pub use search::*;
pub use terminal::*;

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
    ChangeStart,
    ChangeEnd,
    ChangeRepeat,
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
