use log::*;
use crossterm::{
    cursor,
    queue, style,
    terminal::{self, ClearType},
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LineFormat(pub LineFormatType, pub String);

#[derive(Debug)]
pub enum DrawCommand {
    Clear(usize, usize),
    Line(u16, usize, String),
    Row(u16, u16, String),
    Status(u16, String),
    Cursor(u16, u16),
    Format(usize, usize, usize, Vec<LineFormat>),
    SavePosition,
    RestorePosition
}

#[derive(Debug, Clone)]
pub struct RenderCursor {
    pub cx: usize,
    pub cy: usize,
    dirty: bool
}
impl Default for RenderCursor {
    fn default() -> Self {
        Self { cx:0, cy:0, dirty: true }
    }
}
impl RenderCursor {
    pub fn update(&mut self, cx: usize, cy: usize) {
        if self.cx != cx {
            self.cx = cx;
            self.dirty = true;
        }
        if self.cy != cy {
            self.cy = cy;
            self.dirty = true;
        }
    }
    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        if self.dirty {
            vec![DrawCommand::Cursor(self.cx as u16, self.cy as u16)]
        } else {
            vec![]
        }
    }

    pub fn clear(&mut self) {
        self.dirty = true;
    }
}

// render block should be able to handle all text orientations
// this is the abstraction that handles all 8 text orientations
// From an api perspective it's Right-Left, Top-Bottom, but that
// will be possible to change in the future
#[derive(Debug, Clone)]
pub struct RenderBlock {
    pub w: usize,    // width of the block
    pub h: usize,    // height of the block
    pub x0: usize,   // x-coordinate of the top corner
    pub y0: usize,   // y-coordinate of the top corner
    rows: Vec<RowUpdate>
}

impl Default for RenderBlock {
    fn default() -> Self {
        Self { w:0, h:0, x0:0, y0:0, rows: vec![]}
    }
}

impl RenderBlock {
    fn new(&mut self, w: usize, h: usize, x0: usize, y0: usize)  -> Self {
        let mut rows = Vec::new();
        rows.resize_with(self.h, RowUpdate::default);
        Self { w, h, x0, y0, rows }
    }

    pub fn clear(&mut self) {
        self.rows.truncate(0);
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        // reset everything on resize
        self.rows.truncate(0);
        self.rows.resize_with(self.h, RowUpdate::default);
    }

    pub fn update_rows(&mut self, rows: Vec<RowUpdate>) {
        if rows.len() != self.rows.len() {
            error!("Rows mismatch {}/{}", rows.len(), self.rows.len());
        }
        info!("update_rows {:?}", (rows.len(), self.rows.len()));
        self.rows.resize_with(rows.len(), RowUpdate::default);
        self.rows.iter_mut().zip(rows.iter()).enumerate().for_each(|(i, (left, right))| {
            if left != right {
                info!("REP1:{:?}", (&left, &right));
                left.dirty = true;
                left.item = right.item.clone();
            }
        });
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let y0 = self.y0;
        let x0 = self.x0;
        let w = self.w;
        let mut cs: Vec<DrawCommand> = self.rows.iter_mut().enumerate().filter_map(|(inx, r)| {
            if r.dirty {
                r.dirty = false;
                return Some(DrawCommand::Format(x0, y0 + inx, w, r.to_line_format()));
            }
            None
        }).collect();
        if cs.len() > 0 {
            cs.insert(0, DrawCommand::SavePosition);
            cs.push(DrawCommand::RestorePosition);
        }
        cs
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
    info!("C: {:?}", commands.len());
    if commands.len() == 0 {
        return;
    }

    queue!(out,
        //cursor::SavePosition,
        cursor::Hide,
    ).unwrap();
    for command in commands {
        handle_command(out, &command);
    }
    queue!(out,
        //cursor::RestorePosition,
        cursor::Show,
    ).unwrap();
    out.flush().unwrap();
}

pub fn render_flush(out: &mut Stdout) {
    out.flush().unwrap();
}

fn handle_command(out: &mut Stdout, command: &DrawCommand) {
    use DrawCommand::*;
    use LineFormatType::*;

    match command {
        SavePosition => {
            queue!(out, cursor::SavePosition).unwrap();
        }
        RestorePosition => {
            queue!(out, cursor::RestorePosition).unwrap();
        }
        Format(x, y, w, formats) => {
            debug!("F:{:?}", (x, y, w, formats));
            let s = format!("{:empty$}", " ", empty=w);
            queue!(out,
                cursor::MoveTo(*x as u16, *y as u16),
                style::Print(s),
                cursor::MoveTo(*x as u16, *y as u16),
                //terminal::Clear(ClearType::CurrentLine),
            ).unwrap();
            for f in formats.iter() {
                let s = f.1.clone();
                //let s = format!("{:empty$}", f.1, empty=w);
                match f.0 {
                    Normal => queue!(out, style::Print(s)).unwrap(),
                    Highlight => queue!(out, style::Print(s.negative())).unwrap(),
                    Dim => queue!(out, style::Print(s.dim())).unwrap(),
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
            info!("Cursor: {:?}", (a, b));
            queue!(out,
                cursor::MoveTo(*a, *b),
            ).unwrap();
        }
    }
}

