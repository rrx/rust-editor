use log::*;
use ropey::Rope;
use super::*;
use std::io;
use std::fs::File;
use std::path::Path;

use crossbeam::thread;
use crossbeam::channel;
use std::convert::TryInto;
use std::sync::Arc;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Buffer {
    text: Rope,
    spec: Arc<ViewSpec>,
    cursor: Cursor,
    start: Cursor,
    pub path: String
}
impl Buffer {
    pub fn new(text: Rope, spec: Arc<ViewSpec>) -> Self {
        let cursor = cursor_start(&text, spec.sx as usize);
        let start = cursor.clone();
        Self {text, spec, path: "".into(), cursor, start}
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = String::from(path);
    }

    fn insert_char(&mut self, ch: char) {
        let c = self.text.line_to_char(self.cursor.line_inx) + self.cursor.rx + 1;
        self.text.insert_char(c - 1, ch);
        self.cursor = cursor_from_char(&self.text, self.spec.sx as usize, c);
        info!("I: {:?}", (&self.cursor, c));
    }

    pub fn save(&self) {
        let f = File::create(&self.path).unwrap();
        self.text.write_to(f).unwrap();
        //info!("S: {:?}", (&self.text, &self.path));
        info!("Wrote: {} bytes to {}", self.text.len_bytes(), &self.path);
    }

    fn render_line(&self, line_inx: usize) -> Line {
        let lc0 = self.text.line_to_char(line_inx);
        let s = self.text.line(line_inx).to_string();
        Line::new(line_inx, s, self.spec.sx, lc0)
    }

    pub fn update_view(&mut self) -> Vec<DrawCommand> {
        let (start, commands) = LineWorker::render(self.text.clone(), &self.spec, self.start.clone(), self.cursor.clone());
        //info!("R: {:?}", (&start, &self.cursor));
        self.start = start;
        commands
    }

    //pub fn jump_to_line(&mut self, line: i64) {
        //// 0 is the start
        //// negative lines is the number of lines from the end of the file
        //let lines: usize = self.text.len_lines() - 1;
        //let mut line_inx = line as usize;
        //if line < 0 {
            //line_inx = lines - i64::abs(line) as usize;
        //}

        //if line_inx > lines {
            //line_inx = lines;
        //}


        //self.cursor.line_inx = line_inx;
        //self.cursor.cx = 0;
        //self.cursor.rx = 0;
    //}

    pub fn command(&mut self, c: &Command) {
        use Command::*;
        match c {
            Insert(x) => {
                self.insert_char(*x);
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.cursor = cursor_from_line_wrapped(&self.text, self.spec.sx as usize, line_inx);
            }
            MoveCursorX(dx) => {
                self.cursor = cursor_to_relative_x(&self.text, self.spec.sx as usize, &self.cursor, *dx);
                info!("Y: {:?}", (&self.cursor));
            }
            MoveCursorY(dy) => {
                self.cursor = LineWorker::move_y(self.text.clone(), self.spec.sx as usize, self.cursor.clone(), *dy);
                info!("Y: {:?}", (&self.cursor));
            }
            _ => ()
        }
    }
}


