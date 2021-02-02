use crossterm::{
    tty::IsTty,
    cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture,
        },
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{io::{Write, stdout, stdin}};

use crate::text::terminal::*;
use crate::text::display::*;
use crate::ism::{FrontendTrait, process};

pub struct FrontendCrossterm {
    out: std::io::Stdout
}

impl FrontendCrossterm {
    pub fn new() -> Self {
        if !stdin().is_tty() {
            panic!("Not a tty");
        }

        Self {
            out: stdout()
        }
    }

    pub fn read_loop(&mut self, buf: &mut crate::text::TextBuffer) {
        // set initial size
        let (sx, sy) = terminal::size().unwrap();
        buf.set_size(sx, sy);

        enable_raw_mode().unwrap();
        execute!(self.out, EnableMouseCapture).unwrap();
        process(self, buf);
        execute!(self.out, DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
    }
}


impl FrontendTrait for FrontendCrossterm {
    fn reset(&mut self) {
        queue!(self.out,
            style::ResetColor,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0,0)
        ).unwrap();
        self.out.flush().unwrap();
    }

    fn render(&mut self, commands: Vec<DrawCommand>) {
        render_commands(&mut self.out, commands);
    }
}

