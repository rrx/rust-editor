use log::*;
use ropey::Rope;
use super::*;
use std::fs::File;
use crossbeam::channel;

#[derive(Debug, Clone)]
pub struct Buffer {
    pub text: Rope,
    sx: usize,
    sy: usize,
    pub x0: usize,
    pub y0: usize,
    pub cx: usize,
    pub cy: usize,
    cursor: Cursor,
    start: Cursor,
    rows: Vec<RowItem>,
    updates: Vec<RowUpdate>,
    search_results: SearchResults,
    pub path: String
}
impl Buffer {
    pub fn save_text(path: &String, text: &Rope) {
        let f = File::create(path).unwrap();
        text.write_to(f).unwrap();
        info!("Wrote: {} bytes to {}", text.len_bytes(), path);
    }

    pub fn new(text: Rope, sx: usize, sy: usize, x0: usize, y0: usize) -> Self {
        let cursor = cursor_start(&text, sx);
        let start = cursor.clone();
        Self {
            text, sx, sy, x0, y0, path: "".into(), cursor, start,
            rows: Vec::new(), updates: Vec::new(),
            cx: 0, cy: 0,
            search_results: SearchResults::default()
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = String::from(path);
    }

    fn remove_char(&mut self) {
        let c = self.cursor.c;
        if c > 0 {
            self.text.remove(c-1..c);
            self.cursor = cursor_from_char(&self.text, self.sx, c - 1, 0)
                .save_x_hint(self.sx);
        }
        info!("R: {:?}", (&self.cursor, c));
    }
    fn insert_char(&mut self, ch: char) {
        let c = self.cursor.c;
        self.text.insert_char(c, ch);
        self.cursor = cursor_from_char(&self.text, self.sx, c + 1, 0)
            .save_x_hint(self.sx);
        info!("I: {:?}", (&self.cursor, c));
    }

    pub fn save(&self) {
        let f = File::create(&self.path).unwrap();
        self.text.write_to(f).unwrap();
        info!("Wrote: {} bytes to {}", self.text.len_bytes(), &self.path);
    }

    pub fn get_updates(&self) -> &Vec<RowUpdate> {
        &self.updates
    }
    pub fn get_rows(&self) -> &Vec<RowItem> {
        &self.rows
    }

    fn rows_update(&mut self, rows: Vec<RowItem>) {
        self.rows = rows;
        self.updates = self.rows.iter().map(|r| {
            let mut u = RowUpdate::default();
            u.item = RowUpdateType::Row(r.clone());
            u
        }).collect();
        while self.updates.len() < self.sy {
            self.updates.push(RowUpdate::default());
        }
        info!("rows_update: {:?}", (self.rows.len(), self.updates.len()));
    }

    pub fn get_window_updates(&self) -> Vec<EditorWindowUpdate> {
        let mut out = Vec::new();
        out.push(EditorWindowUpdate::Left(self.left_updates()));
        out.push(EditorWindowUpdate::Header(self.header_updates()));
        out.push(EditorWindowUpdate::Status(self.status_updates()));
        out.push(EditorWindowUpdate::Command(self.command_updates()));
        out.push(EditorWindowUpdate::Main(self.get_updates().clone()));
        out.push(EditorWindowUpdate::Cursor(self.cx + self.x0, self.cy + self.y0));
        out
    }

    pub fn send_updates(&self, tx: &channel::Sender<EditorWindowUpdate>) {
        self.get_window_updates().iter().for_each(|u| {
            tx.send(u.clone()).unwrap();
        });
    }

    pub fn header_updates(&self) -> Vec<RowUpdate> {
        let s = format!("Rust-Editor-{} {:width$}", clap::crate_version!(), self.cursor.simple_format(), width=self.sx);
        vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]
    }

    pub fn command_updates(&self) -> Vec<RowUpdate> {
        vec![RowUpdate::from(LineFormat(LineFormatType::Normal, format!("CMD{:width$}", width=self.sx)))]
    }

    pub fn status_updates(&self) -> Vec<RowUpdate> {
        let s = format!(
            "DEBUG: [{},{}] S:{} C:{} {:?} {:width$}",
            self.cx, self.cy,
            &self.start.simple_format(),
            &self.cursor.simple_format(),
            (self.sx, self.sy, self.x0, self.y0),
            " ",
            width=self.sx);
        vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]
    }

    pub fn left_updates(&self) -> Vec<RowUpdate> {
        let mut out = self.rows.iter().enumerate().map(|(inx, row)| {
            let mut line_display = 0; // zero means leave line blank
            if row.cursor.wrap0 == 0 || inx == 0 {
                line_display = row.cursor.line_inx + 1; // display one based
            }
            let fs;
            if line_display > 0 {
                fs = format!("{:5}\u{23A5}", line_display)
            } else {
                fs = format!("{:5}\u{23A5}", " ")
            }
            RowUpdate::from(LineFormat(LineFormatType::Dim, fs))
        }).collect::<Vec<RowUpdate>>();
        while out.len() < self.sy {
            out.push(RowUpdate::default());
        }
        out
    }

    pub fn update_view(&mut self) {
        let (cx, cy, rows) = LineWorker::screen_from_cursor(&self.text, self.sx, self.sy, &self.start, &self.cursor);
        self.cx = cx as usize;
        self.cy = cy as usize;

        info!("update: {:?}", (cx, cy, rows.len()));
        let start = rows[0].cursor.clone();
        self.start = start;
        self.rows_update(rows);
    }

    pub fn update_from_start(&mut self) {
        self.rows_update(LineWorker::screen_from_start(&self.text, self.sx, self.sy, &self.start, &self.cursor));
        let (cx, cy, cursor) = self.locate_cursor_pos_in_window(&self.rows);
        info!("start: {:?}", (cx, cy, self.rows.len()));
        self.cx = cx as usize;
        self.cy = cy as usize;
        self.cursor = cursor;
    }

    pub fn locate_cursor_pos_in_window(&self, rows: &Vec<RowItem>) -> (u16, u16, Cursor) {
        let end = rows.len() - 1;
        if self.cursor < rows[0].cursor {
            (0, 0, rows[0].cursor.clone())
        } else if self.cursor.c >= rows[end].cursor.lc1 {
            (0, end as u16, rows[end].cursor.clone())
        } else {
            let (rx, mut ry) = (0, 0);
            (0..rows.len()).for_each(|i| {
                if self.cursor.line_inx == rows[i].cursor.line_inx && self.cursor.wrap0 == rows[i].cursor.wrap0 {
                    ry = i;
                }
            });
            (rx, ry as u16, rows[ry].cursor.clone())
        }
    }

    pub fn remove_range(&mut self, dx: i32) {
        self.cursor = cursor_remove_range(&mut self.text, self.sx, &self.cursor, dx);
    }

    pub fn scroll(&mut self, dy: i32) {
        self.start = cursor_move_to_y(&self.text, self.sx, &self.start,  dy);
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        self.sx = w;
        self.sy = h;
        self.x0 = x0;
        self.y0 = y0;
        // update cursors which depend on the width
        self.cursor = cursor_from_char(&self.text, w, self.cursor.c, self.cursor.x_hint);
        self.start = cursor_from_char(&self.text, w, self.start.c, self.start.x_hint);
        self.update_view();
    }

    fn cursor_from_xy(&self, mx: usize, my: usize) -> Option<Cursor> {
        let x0 = self.x0;
        let y0 = self.y0;
        let x1 = x0 + self.sx;
        let y1 = y0 + self.sy;
        if self.rows.len() > 0 && mx >= x0  && mx < self.sx && my >= y0 && my < y1 {
            let cx = mx as usize - x0 as usize;
            let cy = my as usize - y0 as usize;
            let mut y = cy;
            if cy >= self.rows.len() {
                y = self.rows.len() - 1;
            }
            let mut c = self.rows[y as usize].cursor.clone();
            c = cursor_to_line_relative(&self.text, self.sx as usize, &c, c.wrap0, cx);
            Some(c)
        } else {
            None
        }
    }

    pub fn cursor_motion(&self, m: &Motion, repeat: usize) -> Cursor {
        let r = repeat as i32;
        let text = &self.text;
        let sx = self.sx;
        let cursor = &self.cursor;
        match m {
            Motion::Left => cursor_move_to_x(text, sx, cursor, -r),
            Motion::Right => cursor_move_to_x(text, sx, cursor, r),
            Motion::Up => cursor_move_to_y(text, sx, cursor, -r),
            Motion::Down => cursor_move_to_y(text, sx, cursor, r),
            Motion::BackWord1 => cursor_move_to_word(text, sx, cursor, -r, false),
            Motion::BackWord2 => cursor_move_to_word(text, sx, cursor, -r, true),
            Motion::ForwardWord1 => cursor_move_to_word(text, sx, cursor, r, false),
            Motion::ForwardWord2 => cursor_move_to_word(text, sx, cursor, r, true),
            Motion::ForwardWordEnd1 => cursor_move_to_word(text, sx, cursor, r, false),
            Motion::ForwardWordEnd2 => cursor_move_to_word(text, sx, cursor, r, true),
            Motion::NextSearch => self.search_reps(&self.cursor, r),
            Motion::PrevSearch => self.search_reps(&self.cursor, -r),
            _ => cursor.clone()
        }
    }

    pub fn search_reps(&self, cursor: &Cursor, reps: i32) -> Cursor {
        self.search_results.next_cursor(&self.text, self.sx, cursor, reps)
    }

    pub fn search_next(&self, reps: i32) -> Cursor {
        match self.search_results.next_from_position(self.cursor.c, reps) {
            Some(sub) => {
                cursor_from_char(&self.text, self.sx, sub.start(), 0)
            }
            None => self.cursor.clone()
        }
    }

    pub fn command(&mut self, c: &Command) {// -> Vec<DrawCommand> {
        use Command::*;
        //let sx = self.sx;
        match c {
            Insert(x) => {
                self.insert_char(*x);
                self.update_view();
            }
            //Backspace => {
                //self.remove_char();
                //self.update_view();
            //}
            RemoveChar(dx) => {
                self.remove_range(*dx);
                self.update_view();
            }
            ScrollPage(ratio) => {
                let xdy = self.sy as f32 / *ratio as f32;
                self.scroll(xdy as i32);
                self.update_from_start();
            }
            Scroll(dy) => {
                self.scroll(*dy as i32);
                self.update_from_start();
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.cursor = cursor_from_line_wrapped(&self.text, self.sx, line_inx);
                self.update_view();
            }
            LineNav(dx) => {
                self.cursor = cursor_move_to_lc(&self.text, self.sx, &self.cursor, *dx)
                    .save_x_hint(self.sx);
                self.update_view();
            }
            Resize(x, y) => {
                self.resize(*x as usize, *y as usize, self.x0, self.y0);
                self.update_view();
            }

            Motion(reps, m) => {
                self.cursor = self.cursor_motion(m, *reps);
                self.update_view();
            }

            //Search(s) => {
                //self.search_results = SearchResults::new_search(&self.text, s.as_str(), false);
                //self.cursor = self.search_next(1);
                //self.update_view();
            //}

            Mouse(x, y) => {
                match self.cursor_from_xy(*x as usize, *y as usize) {
                    Some(c) => {
                        self.cursor = c;
                        self.update_view();
                    }
                    _ => ()
                }
            }
            _ => ()//Vec::new()
        }
    }
}


