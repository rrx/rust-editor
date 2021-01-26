use log::*;
use ropey::Rope;
use super::*;
use std::fs::File;

use std::sync::Arc;

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

    fn remove_char(&mut self) {
        let sx = self.spec.sx as usize;
        let c = self.cursor.c;
        if c > 0 {
            self.text.remove(c-1..c);
            self.cursor = cursor_from_char(&self.text, sx, c - 1, 0);
            self.cursor.save_x_hint(sx);
        }
        info!("R: {:?}", (&self.cursor, c));
    }
    fn insert_char(&mut self, ch: char) {
        let sx = self.spec.sx as usize;
        //let rx = self.cursor.rx(sx);
        //let c = self.text.line_to_char(self.cursor.line_inx) + rx + 1;
        let c = self.cursor.c;
        self.text.insert_char(c, ch);
        self.cursor = cursor_from_char(&self.text, sx, c + 1, 0);
        self.cursor.save_x_hint(sx);
        info!("I: {:?}", (&self.cursor, c));
    }

    pub fn save(&self) {
        let f = File::create(&self.path).unwrap();
        self.text.write_to(f).unwrap();
        //info!("S: {:?}", (&self.text, &self.path));
        info!("Wrote: {} bytes to {}", self.text.len_bytes(), &self.path);
    }

    //fn render_line(&self, line_inx: usize) -> Line {
        //let lc0 = self.text.line_to_char(line_inx);
        //let s = self.text.line(line_inx).to_string();
        //Line::new(line_inx, s, self.spec.sx, lc0)
    //}

    pub fn update_view(&mut self) -> Vec<DrawCommand> {
        let (start, commands) = LineWorker::render(&self.text, &self.spec, &self.start, &self.cursor);
        //info!("R: {:?}", (&start, &self.cursor));
        self.start = start;
        commands
    }

    pub fn update_from_start(&mut self) -> Vec<DrawCommand> {
        let rows = LineWorker::screen_from_start(&self.text, self.spec.sx as usize, self.spec.sy as usize, &self.start, &self.cursor);
        let (cx, cy, cursor) = self.locate_cursor_pos_in_window(&rows);
        self.cursor = cursor;
        LineWorker::render_rows(&self.text, &self.spec, cx, cy, &rows, &self.cursor)
    }

    pub fn locate_cursor_pos_in_window(&self, rows: &Vec<RowItem>) -> (u16, u16, Cursor) {
        if rows.len() == 0 {
            (0, 0, self.cursor.clone())
        } else if self.cursor < rows[0].cursor {
            (0, 0, rows[0].cursor.clone())
        } else if self.cursor.c >= rows[rows.len() - 1].cursor.lc1 {
            let end = rows.len() - 1;
            (0, end as u16, rows[end].cursor.clone())
        } else {
            let (mut rx, mut ry) = (0, 0);
            (0..rows.len()).for_each(|i| {
                if self.cursor.line_inx == rows[i].cursor.line_inx && self.cursor.wrap0 == rows[i].cursor.wrap0 {
                    ry = i;
                }
            });
            (rx, ry as u16, rows[ry].cursor.clone())
        }
    }

    pub fn remove_range(&mut self, dx: i32) {
        let mut start = 0;
        let mut end = 0;
        if dx < 0 {
            start = self.cursor.c as i32 + dx;
            if start < 0 {
                start = 0;
            }
            end = self.cursor.c as i32;
        } else if dx > 0 {
            start = self.cursor.c as i32;
            end = self.cursor.c as i32 + dx;
            if end > self.text.len_chars() as i32 - 1 {
                end = self.text.len_chars() as i32 - 1;
            }
        }

        if start != end {
            let sx = self.spec.sx as usize;
            self.text.remove(start as usize .. end as usize);
            self.cursor = cursor_from_char(&self.text, sx, start as usize, 0);
            self.cursor.save_x_hint(sx);
        }
    }

    pub fn scroll(&mut self, dy: i32) {
        self.start = LineWorker::move_y(&self.text, self.spec.sx as usize, &self.start,  dy);
    }

    pub fn command(&mut self, c: &Command) -> Vec<DrawCommand> {
        use Command::*;
        match c {
            Insert(x) => {
                self.insert_char(*x);
                self.update_view()
            }
            Backspace => {
                self.remove_char();
                self.update_view()
            }
            RemoveChar(dx) => {
                self.remove_range(*dx);
                self.update_view()
            }
            ScrollPage(ratio) => {
                let xdy = self.spec.sy as f32 / *ratio as f32;
                self.scroll(xdy as i32);
                self.update_from_start()
            }
            Scroll(dy) => {
                self.scroll(*dy as i32);
                self.update_from_start()
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.cursor = cursor_from_line_wrapped(&self.text, self.spec.sx as usize, line_inx);
                self.update_view()
            }
            LineNav(dx) => {
                self.cursor = cursor_move_to_lc(&self.text, self.spec.sx as usize, &self.cursor, *dx);
                self.cursor.save_x_hint(self.spec.sx as usize);
                self.update_view()
            }
            MoveCursorX(dx) => {
                self.cursor = cursor_move_to_x(&self.text, self.spec.sx as usize, &self.cursor, *dx);
                self.cursor.save_x_hint(self.spec.sx as usize);
                self.update_view()
                //info!("Y: {:?}", (&self.cursor));
            }
            MoveCursorY(dy) => {
                self.cursor = LineWorker::move_y(&self.text, self.spec.sx as usize, &self.cursor, *dy);
                self.update_view()
                //info!("Y: {:?}", (&self.cursor));
            }
            _ => Vec::new()
        }
    }
}


