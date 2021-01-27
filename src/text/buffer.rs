use log::*;
use ropey::Rope;
use super::*;
use std::fs::File;

//use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Buffer {
    text: Rope,
    spec: ViewSpec,
    cursor: Cursor,
    start: Cursor,
    rows: Vec<RowItem>,
    search_results: SearchResults,
    pub path: String
}
impl Buffer {
    pub fn new(text: Rope, spec: ViewSpec) -> Self {
        let cursor = cursor_start(&text, spec.sx as usize);
        let start = cursor.clone();
        Self {
            text, spec, path: "".into(), cursor, start, rows: Vec::new(),
            search_results: SearchResults::default()
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = String::from(path);
    }

    fn remove_char(&mut self) {
        let sx = self.spec.sx as usize;
        let c = self.cursor.c;
        if c > 0 {
            self.text.remove(c-1..c);
            self.cursor = cursor_from_char(&self.text, sx, c - 1, 0)
                .save_x_hint(sx);
        }
        info!("R: {:?}", (&self.cursor, c));
    }
    fn insert_char(&mut self, ch: char) {
        let sx = self.spec.sx as usize;
        //let rx = self.cursor.rx(sx);
        //let c = self.text.line_to_char(self.cursor.line_inx) + rx + 1;
        let c = self.cursor.c;
        self.text.insert_char(c, ch);
        self.cursor = cursor_from_char(&self.text, sx, c + 1, 0)
            .save_x_hint(sx);
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
        let sx = self.spec.sx as usize;
        let sy = self.spec.sy as usize;
        let (cx, cy, rows) = LineWorker::screen_from_cursor(&self.text, sx, sy, &self.start, &self.cursor);
        let commands = LineWorker::render_rows(&self.text, &self.spec, cx, cy, &rows, &self.cursor);
        let start = rows[0].cursor.clone();
        self.start = start;
        self.rows = rows;
        //(start, commands)
        //let (start, commands) = LineWorker::render(&self.text, &self.spec, &self.start, &self.cursor);
        //info!("R: {:?}", (&start, &self.cursor));
        commands
    }

    pub fn update_from_start(&mut self) -> Vec<DrawCommand> {
        self.rows = LineWorker::screen_from_start(&self.text, self.spec.sx as usize, self.spec.sy as usize, &self.start, &self.cursor);
        let (cx, cy, cursor) = self.locate_cursor_pos_in_window(&self.rows);
        self.cursor = cursor;
        LineWorker::render_rows(&self.text, &self.spec, cx, cy, &self.rows, &self.cursor)
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
            self.cursor = cursor_from_char(&self.text, sx, start as usize, 0)
                .save_x_hint(sx);
        }
    }

    pub fn scroll(&mut self, dy: i32) {
        self.start = cursor_move_to_y(&self.text, self.spec.sx as usize, &self.start,  dy);
    }

    fn resize(&mut self, w: u16, h: u16, origin_x: u16, origin_y: u16) {
        self.spec.resize(w, h, origin_x, origin_y);
    }

    fn cursor_from_xy(&self, mx: u16, my: u16) -> Cursor {
        let ViewSpec { x0, y0, sx, sy, ..} = self.spec;
        let x1 = x0 + sx;
        let y1 = y0 + sy;
        if mx >= x0  && mx < sx && my >= y0 && my < y1 {
            let cx = mx as usize - x0 as usize;
            let cy = my as usize - y0 as usize;
            //let mut c = self.cursor.clone();
            let mut y = cy;
            if cy >= self.rows.len() {
                y = self.rows.len() - 1;
            }
            let mut c = self.rows[y as usize].cursor.clone();
            c = cursor_to_line_relative(&self.text, self.spec.sx as usize, &c, c.wrap0, cx);
            c
        } else {
            self.cursor.clone()
        }
    }

    pub fn cursor_motion(&self, m: &Motion, repeat: usize) -> Cursor {
        let r = repeat as i32;
        let text = &self.text;
        let sx = self.spec.sx as usize;
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
        let sx = self.spec.sx as usize;
        let mut count = 0;
        let mut c = cursor.c;
        let end = i32::abs(reps);
        while count < end {
            let result;
            if reps < 0 {
                result = self.search_results.prev_from_position(c);
            } else if reps > 0 {
                result = self.search_results.next_from_position(c);
            } else {
                break;
            }

            match result {
                Some(sub) => {
                    c = sub.start();
                }
                None => break
            }
            count += 1;
        }
        if c != cursor.c {
            cursor_from_char(&self.text, sx, c, 0)
        } else {
            cursor.clone()
        }
    }

    pub fn search_next(&self, reps: usize) -> Cursor {
        let sx = self.spec.sx as usize;
        match self.search_results.next_from_position(self.cursor.c) {
            Some(sub) => {
                cursor_from_char(&self.text, sx, sub.start(), 0)
            }
            None => self.cursor.clone()
        }
    }

    pub fn command(&mut self, c: &Command) -> Vec<DrawCommand> {
        use Command::*;
        let sx = self.spec.sx as usize;
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
                self.cursor = cursor_move_to_lc(&self.text, self.spec.sx as usize, &self.cursor, *dx)
                    .save_x_hint(self.spec.sx as usize);
                self.update_view()
            }
            //MoveCursorX(dx) => {
                //self.cursor = cursor_move_to_x(&self.text, self.spec.sx as usize, &self.cursor, *dx);
                //self.cursor.save_x_hint(self.spec.sx as usize);
                //self.update_view()
            //}
            //MoveCursorY(dy) => {
                //self.cursor = LineWorker::move_y(&self.text, self.spec.sx as usize, &self.cursor, *dy);
                //self.update_view()
            //}
            Resize(x, y) => {
                self.resize(*x, *y, 0, 0);
                self.update_view()
            }

            Motion(reps, m) => {
                self.cursor = self.cursor_motion(m, *reps);
                self.update_view()
            }

            Search(s) => {
                self.search_results = SearchResults::new_search(&self.text, s.as_str());
                self.cursor = self.search_next(1);
                self.update_view()
            }

            Mouse(x, y) => {
                self.cursor = self.cursor_from_xy(*x, *y);
                self.update_view()
            }
            _ => Vec::new()
        }
    }
}


