use std::time::Duration;
use crossterm::{
    event::{
        poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent,
        KeyModifiers, MouseEvent, MouseEventKind},
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{io::{self, Write, stdout, stdin}};

use crate::frontend::{read_loop, FrontendTrait, DrawCommand};

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
        execute!(self.out, EnableMouseCapture);
        read_loop(self, buf);
        execute!(self.out, DisableMouseCapture);
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



