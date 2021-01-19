use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture,
        },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::stdout;

use crate::frontend::DrawCommand;
use crate::ism::{FrontendTrait, read_loop, InputStateMachine};

pub struct FrontendDebug {
    out: std::io::Stdout
}

impl FrontendDebug {
    pub fn new() -> Self {
        Self {
            out: stdout()
        }
    }
    pub fn read_loop(&mut self, buf: &mut crate::text::TextBuffer) {
        enable_raw_mode().unwrap();
        execute!(self.out, EnableMouseCapture).unwrap();
        read_loop(self, buf);
        execute!(self.out, DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
    }
}

impl FrontendTrait for FrontendDebug {
    fn reset(&mut self) {
    }
    fn render(&mut self, commands: Vec<DrawCommand>) {
        for c in commands {
            println!("{:?}", c);
        }
    }
}



