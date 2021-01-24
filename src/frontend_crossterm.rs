use log::*;
use crossterm::{
    tty::IsTty,
    cursor,
    event::{
        DisableMouseCapture, EnableMouseCapture,
        },
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{io::{Write, stdout, stdin, Stdout}};
use crossterm::style::Styler;

use crate::frontend::DrawCommand;
use crate::ism::{FrontendTrait, read_loop, process, InputStateMachine};

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

pub fn render_reset(out: &mut Stdout) {
    queue!(out,
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0,0)
    ).unwrap();
    out.flush().unwrap();
}

pub fn render_commands(out: &mut Stdout, commands: Vec<DrawCommand>) {
    queue!(out,
        cursor::Hide,
    ).unwrap();
    //info!("C: {:?}", commands.len());
    for command in commands {
        handle_command(out, &command);
    }
    queue!(out,
        cursor::Show,
    ).unwrap();
    out.flush().unwrap();
}

fn handle_command(out: &mut Stdout, command: &DrawCommand) {
    match command {
        DrawCommand::Status(row, s) => {
            //info!("S: {:?}", (row, &s));
            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s.clone().negative())
            ).unwrap();
        },

        DrawCommand::Row(x, y, s) => {
            queue!(out,
                cursor::MoveTo(*x, *y),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(s),
                //terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();
        }

        DrawCommand::Line(row, line, s) => {
            let fs;
            if *line > 0 {
                fs = format!("{:5} {}", line, s)
            } else {
                fs = format!("{:5} {}", " ", s)
            }

            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
                style::Print(fs)
            ).unwrap();
        },
        DrawCommand::Clear(row) => {
            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
            ).unwrap();
        }
        DrawCommand::Cursor(a, b) => {
            //info!("Cursor: {:?}", (a, b));
            queue!(out,
                cursor::MoveTo(*a, *b),
            ).unwrap();
        }
    }
}
