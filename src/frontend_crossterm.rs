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
use std::{io::{Write, stdout, stdin}};
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

    //pub fn process<'a>(&mut self, buf: &'a mut crate::SmartBuffer) {
        //// set initial size
        //let (sx, sy) = terminal::size().unwrap();
        //let mut app = crate::App::new(&'a buf, sx, sy);

        //enable_raw_mode().unwrap();
        //execute!(self.out, EnableMouseCapture).unwrap();
        //app.process(self);
        //execute!(self.out, DisableMouseCapture).unwrap();
        //disable_raw_mode().unwrap();
    //}
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
        queue!(self.out,
            cursor::Hide,
        ).unwrap();
        info!("C: {:?}", commands.len());
        for command in commands {
            match command {
                DrawCommand::Status(row, s) => {
                    info!("S: {:?}", (row, &s));
                    queue!(self.out,
                        cursor::MoveTo(0, row),
                        terminal::Clear(ClearType::CurrentLine),
                        style::Print(s.negative())
                    ).unwrap();
                },

                DrawCommand::Row(x, y, s) => {
                    queue!(self.out,
                        cursor::MoveTo(x, y),
                        terminal::Clear(ClearType::CurrentLine),
                        style::Print(s),
                        //terminal::Clear(ClearType::UntilNewLine),
                    ).unwrap();
                }

                DrawCommand::Line(row, line, s) => {
                    let fs;
                    if line > 0 {
                        fs = format!("{:5} {}", line, s)
                    } else {
                        fs = format!("{:5} {}", " ", s)
                    }

                    queue!(self.out,
                        cursor::MoveTo(0, row),
                        terminal::Clear(ClearType::CurrentLine),
                        style::Print(fs)
                    ).unwrap();
                },
                DrawCommand::Clear(row) => {
                    queue!(self.out,
                        cursor::MoveTo(0, row),
                        terminal::Clear(ClearType::CurrentLine),
                    ).unwrap();
                }
                DrawCommand::Cursor(a, b) => {
                    queue!(self.out,
                        cursor::MoveTo(a, b),
                    ).unwrap();
                }
            }
        }
        queue!(self.out,
            cursor::Show,
        ).unwrap();
        self.out.flush().unwrap();
    }
}



