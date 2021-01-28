use log::*;
use crossterm::{
    cursor,
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{io::{Write, Stdout}};
use crossterm::style::Styler;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum LineFormatType {
    Dim,
    Normal,
    Highlight
}

#[derive(Debug)]
pub struct LineFormat(pub LineFormatType, pub String);

#[derive(Debug)]
pub enum DrawCommand {
    Clear(u16),
    Line(u16, usize, String),
    Row(u16, u16, String),
    Status(u16, String),
    Cursor(u16, u16),
    Format(u16, Vec<LineFormat>)
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
    use DrawCommand::*;
    use LineFormatType::*;

    match command {
        Format(row, formats) => {
            info!("F:{:?}", (row, formats));
            queue!(out,
                cursor::MoveTo(0, *row),
                terminal::Clear(ClearType::CurrentLine),
            ).unwrap();
            for f in formats.iter() {
                match f.0 {
                    Normal => queue!(out, style::Print(f.1.clone())).unwrap(),
                    Highlight => queue!(out, style::Print(f.1.clone().negative())).unwrap(),
                    Dim => queue!(out, style::Print(f.1.clone().dim())).unwrap(),
                }
            }
        }

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

