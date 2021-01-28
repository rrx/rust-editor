use log::*;
use crossterm::{
    cursor,
    execute, queue, style,
    terminal::{self, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{io::{Write, Stdout}};
use crossterm::style::Styler;
use super::*;

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
    Clear(usize, usize),
    Line(u16, usize, String),
    Row(u16, u16, String),
    Status(u16, String),
    Cursor(u16, u16),
    Format(usize, usize, Vec<LineFormat>)
}

// render block should be able to handle all text orientations
// this is the abstraction that handles all 8 text orientations
// From an api perspective it's Right-Left, Top-Bottom, but that
// will be possible to change in the future
pub struct RenderBlock {
    w: usize,    // width of the block
    h: usize,    // height of the block
    x0: usize,   // x-coordinate of the top corner
    y0: usize,   // y-coordinate of the top corner
    rows: Vec<Option<RowItem>>
}
impl Default for RenderBlock {
    fn default() -> Self {
        Self { w:0, h:0, x0:0, y0:0, rows: vec![]}
    }
}
impl RenderBlock {
    fn new(&mut self, w: usize, h: usize, x0: usize, y0: usize)  -> Self {
        let mut rows = Vec::new();
        rows.resize_with(self.h, || None);
        Self { w, h, x0, y0, rows }
    }

    pub fn update_view(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        self.rows.resize_with(self.h, || None);
    }

    pub fn update_rows(&mut self, rows: Vec<RowItem>) {
        if rows.len() != self.rows.len() {
            error!("Rows mismatch {}/{}", rows.len(), self.rows.len());
        }
        self.rows = rows.iter().enumerate().map(|(i, row)| {
            //let o = self.rows.get_mut(i);
            //if o.is_none() {
                //return;
            //}
            //let w = o.unwrap();
            Some(row.clone())

            //match w {
                //Some(r0) => {
                    //if r0.elements != row.elements {
                        //r0.elements = row.elements.clone();
                        //r0.dirty = true;
                    //}
                //}
                //None => {
                    //w.replace(row.clone());
                //}
            //}
        }).collect();
        while self.rows.len() < self.h {
            self.rows.push(None);
        }
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let y0 = self.y0;
        let x0 = self.x0;
        self.rows.iter_mut().enumerate().filter_map(|(inx, r)| {
            if let Some(row) = r {
                //if row.dirty {
                    //row.dirty = false;
                    return Some(DrawCommand::Format(x0, y0 + inx, row.to_line_format()));
                //}
            }
            Some(DrawCommand::Format(x0, y0 + inx, vec![]))
        }).collect()
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
    info!("C: {:?}", commands.len());
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
        Format(x, y, formats) => {
            info!("F:{:?}", (x, y, formats));
            queue!(out,
                cursor::MoveTo(*x as u16, *y as u16),
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
        DrawCommand::Clear(x, y) => {
            queue!(out,
                cursor::MoveTo(*x as u16, *y as u16),
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

