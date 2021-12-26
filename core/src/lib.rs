pub mod registers;
pub mod macros;
pub mod config;
pub mod variables;
pub mod buffer;
pub mod graphemes;

pub use macros::{Macros, MacroId};
pub use registers::{Registers, Register};
pub use config::{BufferConfig, EndOfLine, IndentSize, IndentStyle};
pub use variables::{Variable, Variables};
pub use buffer::{Buffer};
pub use graphemes::{RopeGraphemes, grapheme_width};
use ropey::Rope;

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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Motion {
    OnCursor,
    AfterCursor,
    Left,
    Right,
    Up,
    Down,
    // Line
    EOL,
    SOL,
    NextLine,
    SOLT, // start of line text
    Line,
    AbsLine,

    ForwardWord1,
    ForwardWord2,
    ForwardWordEnd1,
    ForwardWordEnd2,
    BackWord1,
    BackWord2,
    NextWord,
    EOW,
    PrevWord,
    SOW,
    NextSearch,
    PrevSearch,
    Til1(char),
    Til2(char),
    // start and end of buffer
    SOB,
    EOB,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Command {
    Insert(String),
    Join,
    Motion(usize, Motion),
    Delete(usize, Motion),
    Yank(Register, Motion),         // register, Motion
    Paste(usize, Register, Motion), // repetitions, register, Motion
    RemoveChar(i32),
    Mode(Mode),
    MacroStart(macros::MacroId),
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
    Open(String),
    SaveAs(String),
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
    Undo,
    Redo,
    Test,
    Refresh,
    Reset,
    VarGet(String),
    VarSet(String, String),
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

