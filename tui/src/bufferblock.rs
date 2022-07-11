use crate::lineworker::LineWorker;
use crate::*;
use editor_core::Buffer;
use editor_core::{BufferConfig, Command, Motion, ViewPos};
use log::*;
use ropey::Rope;

#[derive(Debug, Clone)]
pub struct BufferBlock {
    pub buf: Buffer,
    pub cursor: Cursor,
    pub start: Cursor,
    pub view: ViewPos,
    pub prefix: usize,
    pub rc: RenderCursor,
    pub left: RenderBlock,
    pub block: RenderBlock,
    pub cache_render_rows: Vec<Cursor>,
    search_results: SearchResults,
    is_focused: bool,
}

impl BufferBlock {
    pub fn new(buf: Buffer, view: ViewPos) -> Self {
        let config = buf.get_config();
        let text = buf.get_text();
        let start = cursor_start(&text, view.w, &config);
        let cursor = cursor_start(&text, view.w, &config);

        Self {
            left: RenderBlock::new(view.clone()),
            block: RenderBlock::new(view.clone()),
            cache_render_rows: Vec::new(),
            search_results: SearchResults::default(),
            start,
            cursor,
            view,
            prefix: 0,
            rc: RenderCursor::default(),
            buf,
            is_focused: false,
        }
    }
}

impl BufferBlock {
    pub fn get_text(&self) -> Rope {
        self.buf.get_text()
    }

    pub fn get_config(&self) -> BufferConfig {
        self.buf.get_config().clone()
    }

    pub fn replace_buffer(&mut self, s: &str) -> &mut Self {
        self.buf.replace_buffer(s);
        self
    }

    pub fn set_path(&mut self, s: &str) -> &mut Self {
        self.buf.set_path(s);
        self
    }

    pub fn get_path(&self) -> String {
        self.buf.get_path()
    }

    pub fn update_from_start(&mut self) -> &mut Self {
        let text = self.buf.get_text();
        self.cache_render_rows = LineWorker::screen_from_start(
            &text,
            self.view.w,
            self.view.h,
            &self.start,
            &self.cursor,
        );
        let (cx, cy, cursor) = self.locate_cursor_pos_in_window(&self.cache_render_rows);
        //info!("buffer start: {:?}", (cx, cy, self.cache_render_rows.len()));
        self.rc.update(cx as usize, cy as usize);
        self.cursor = cursor;
        self
    }

    pub fn locate_cursor_pos_in_window(&self, rows: &Vec<Cursor>) -> (u16, u16, Cursor) {
        let end = rows.len() - 1;
        if self.cursor < rows[0] {
            (0, 0, rows[0].clone())
        } else if self.cursor.c >= rows[end].lc1 {
            (0, end as u16, rows[end].clone())
        } else {
            let (rx, mut ry) = (0, 0);
            (0..rows.len()).for_each(|i| {
                if self.cursor.line_inx == rows[i].line_inx && self.cursor.wrap0 == rows[i].wrap0 {
                    ry = i;
                }
            });
            (
                rx + self.block.view.x0 as u16,
                (ry + self.block.view.y0) as u16,
                rows[ry].clone(),
            )
        }
    }

    pub fn update(&mut self) -> &mut Self {
        let text = self.buf.get_text();
        let config = self.buf.get_config();

        // refresh the cursors, which might contain stale data
        self.start = cursor_update(&text, self.view.w, &self.start);
        self.cursor = cursor_update(&text, self.view.w, &self.cursor);

        // render the view, so we know how long the line is on screen
        let (cx, cy, rows) = LineWorker::screen_from_cursor(
            &text,
            self.view.w,
            self.view.h,
            &self.start,
            &self.cursor,
        );
        // update start based on render
        debug!("buffer update: {:?}", (cx, cy, rows.len()));
        let start = rows[0].clone();
        self.start = start;
        // update cursor position
        self.rc.update(
            self.block.view.x0 + cx as usize,
            self.block.view.y0 + cy as usize,
        );

        // generate updates
        let mut updates = rows
            .iter()
            .map(|r| {
                RowUpdate::from_formats(r.to_line_format(
                    &config,
                    self.view.w,
                    self.block.highlight.clone(),
                ))
            })
            .collect::<Vec<RowUpdate>>();
        while updates.len() < self.view.h {
            updates.push(RowUpdate::default());
        }
        self.block.update_rows(updates);

        // update cache rows
        self.cache_render_rows = rows;
        self
    }

    pub fn set_focus(&mut self, f: bool) -> &mut Self {
        self.is_focused = f;
        self
    }

    pub fn update_rows(&mut self, rows: Vec<RowUpdate>) -> &mut Self {
        self.block.update_rows(rows);
        self
    }

    pub fn clear(&mut self) -> &mut Self {
        self.block.clear();
        self.left.clear();
        self.rc.clear();
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = vec![];
        out.append(&mut self.block.generate_commands());
        out.append(&mut self.left.generate_commands());

        // only render the cursor if we are focussed
        if self.is_focused {
            out.append(&mut self.rc.generate_commands());
        }
        //info!("commands: {:?}", &out);
        out
    }

    pub fn resize(&mut self, view: ViewPos, prefix: usize) -> &mut Self {
        self.prefix = prefix;
        let p = if view.w < 10 { 0 } else { prefix };
        self.left.resize(ViewPos {
            w: p,
            h: view.h,
            x0: view.x0,
            y0: view.y0,
        });
        self.block.resize(ViewPos {
            w: view.w - p,
            h: view.h,
            x0: view.x0 + p,
            y0: view.y0,
        });
        let text = self.buf.get_text();
        self.cursor = cursor_resize(&text, view.w, &self.cursor);
        self.start = cursor_resize(&text, view.w, &self.start);
        self.view = view;
        self.clear();
        self
    }

    pub fn remove_range(&mut self, dx: i32) -> &mut Self {
        let mut start = self.cursor.c as i32;
        let mut end = self.cursor.c as i32;
        let sx = self.block.view.w;

        if dx < 0 {
            start += dx;
            if start < 0 {
                start = 0;
            }
        } else if dx > 0 {
            end += dx;
        }

        debug!("remove: {:?}", (sx, dx, start, end));

        if start < end {
            self.buf.remove_range(start as usize, end as usize);
        }

        self.cursor = cursor_from_char(
            &self.buf.get_text(),
            sx,
            &self.cursor.config,
            start as usize,
            0,
        )
        .save_x_hint(sx);

        self
    }

    pub fn insert_string(&mut self, ch: &str) -> &mut Self {
        let length = self.buf.insert_string(self.cursor.c, ch);
        self.cursor = cursor_from_char(
            &self.buf.get_text(),
            self.block.view.w,
            &self.buf.get_config(),
            self.cursor.c + length,
            0,
        )
        .save_x_hint(self.block.view.w);
        //info!("insert: {:?}", (&self.cursor));
        self
    }

    pub fn remove_char(&mut self) -> &mut Self {
        let c = self.cursor.c;
        if c > 0 {
            self.buf.remove_char(c);
            let text = self.buf.get_text();
            let config = self.buf.get_config();
            self.cursor =
                cursor_from_char(&text, self.view.w, &config, c - 1, 0).save_x_hint(self.view.w);
        }
        //info!("remove: {:?}", (&self.cursor, c));
        self
    }

    // remove trailing newlines, to join with the next line
    pub fn join_line(&mut self) -> &mut Self {
        let text = self.buf.get_text();
        let cursor = cursor_update(&text, self.view.w, &self.cursor);
        self.buf.join_line(cursor.line_inx);
        let text = self.buf.get_text();
        self.cursor = cursor_update(&text, self.view.w, &cursor);
        self
    }

    pub fn scroll(&mut self, dy: i32) -> &mut Self {
        let text = self.get_text();
        self.start = cursor_move_to_y(&text, self.view.w, &self.start, dy);
        self
    }

    pub fn cursor_from_xy(&self, mx: usize, my: usize) -> Option<Cursor> {
        let x0 = self.block.view.x0;
        let y0 = self.block.view.y0;
        let y1 = y0 + self.block.view.h;
        let w = self.block.view.w;

        let text = self.buf.get_text();
        let rows = &self.cache_render_rows;
        if rows.len() > 0 && mx >= x0 && mx < w && my >= y0 && my < y1 {
            let cx = mx as usize - x0 as usize;
            let cy = my as usize - y0 as usize;
            let mut y = cy;
            if cy >= rows.len() {
                y = rows.len() - 1;
            }
            let mut c = rows[y as usize].clone();
            c = cursor_to_line_relative(&text, w, &c, c.wrap0, cx);
            Some(c)
        } else {
            None
        }
    }

    pub fn cursor_move_line(&mut self, line_inx: i64) -> &mut Self {
        let text = self.buf.get_text();
        let config = self.buf.get_config();
        self.cursor = cursor_from_line_wrapped(&text, self.view.w, &config, line_inx);
        self
    }

    pub fn cursor_move_lc(&mut self, dx: i32) -> &mut Self {
        let text = self.buf.get_text();
        self.cursor =
            cursor_move_to_lc(&text, self.view.w, &self.cursor, dx).save_x_hint(self.view.w);
        self
    }

    pub fn motion_slice(&mut self, m: &Motion) -> String {
        let (c1, c2) = self.cursor_motion(m, 1);
        let r = if c1.c > c2.c { c2.c..c1.c } else { c1.c..c2.c };
        self.buf.get_text().slice(r).to_string()
    }

    pub fn cursor_motion(&self, m: &Motion, repeat: usize) -> (Cursor, Cursor) {
        let text = self.buf.get_text();
        let r = repeat as i32;
        let sx = self.view.w;
        let cursor = &self.cursor;
        use Motion::*;
        let c1 = cursor.clone();
        let c2 = cursor.clone();
        let config = c1.config.clone();
        match m {
            OnCursor => (c1, c2),
            AfterCursor => (c1, cursor_move_to_x(&text, sx, cursor, 1)),
            Line => {
                let line0 = cursor.line_inx;
                let line1 = cursor.line_inx + 1;
                (
                    cursor_from_line(&text, sx, &config, line0),
                    cursor_from_line(&text, sx, &config, line1),
                )
            }
            EOL => (c1, cursor_move_to_lc(&text, sx, cursor, -1)),
            NextLine => (
                c1,
                cursor_from_line(&text, sx, &config, cursor.line_inx + 1),
            ),
            SOL => (c1, cursor_move_to_lc(&text, sx, cursor, 0)),
            SOLT => (c1, cursor_move_to_lc(&text, sx, cursor, 0)),
            Left => (c1, cursor_move_to_x(&text, sx, cursor, -r)),
            Right => (c1, cursor_move_to_x(&text, sx, cursor, r)),
            Up => (c1, cursor_move_to_y(&text, sx, cursor, -r)),
            Down => (c1, cursor_move_to_y(&text, sx, cursor, r)),
            BackWord1 => (c1, cursor_move_to_word(&text, sx, cursor, -r, false)),
            BackWord2 => (c1, cursor_move_to_word(&text, sx, cursor, -r, true)),
            ForwardWord1 => (c1, cursor_move_to_word(&text, sx, cursor, r, false)),
            ForwardWord2 => (c1, cursor_move_to_word(&text, sx, cursor, r, true)),
            ForwardWordEnd1 => (c1, cursor_move_to_word(&text, sx, cursor, r, false)),
            ForwardWordEnd2 => (c1, cursor_move_to_word(&text, sx, cursor, r, true)),
            NextSearch => (c1, self.search_results.next_cursor(&text, sx, cursor, r)),
            PrevSearch => (c1, self.search_results.next_cursor(&text, sx, cursor, -r)),
            Til1(ch) => (c1, cursor_move_to_char(&text, sx, cursor, r, *ch, false)),
            Til2(ch) => (c1, cursor_move_to_char(&text, sx, cursor, r, *ch, true)),
            _ => (c1, c2),
        }
    }

    pub fn cursor_move(&mut self, cursor: Cursor) -> &mut Self {
        self.cursor = cursor;
        self
    }

    pub fn search(&mut self, s: &str, reverse: bool) -> &mut Self {
        let text = self.buf.get_text();
        self.search_results = SearchResults::new_search(&text, s, reverse);
        self
    }

    pub fn search_next(&mut self, reps: i32) -> &mut Self {
        let text = self.buf.get_text();
        let mut cursor = self.cursor.clone();
        cursor = match self.search_results.next_from_position(cursor.c, reps) {
            Some(sub) => cursor_from_char(&text, self.view.w, &cursor.config, sub.start(), 0),
            None => cursor,
        };
        self.cursor = cursor;
        self
    }

    pub fn paste_motion(&mut self, m: &Motion, s: &String, reps: usize) -> &mut Self {
        let (_, c) = self.cursor_motion(m, 1);
        (0..reps).for_each(|_| {
            self.buf.insert_string(c.c, s.as_str());
        });
        self
    }

    pub fn delete_motion(&mut self, m: &Motion, repeat: usize) -> &mut Self {
        match m {
            Motion::Line => {
                let (start, end) = self.cursor_motion(m, repeat);
                self.buf.delete_line_range(start.line_inx, end.line_inx);
                let text = self.buf.get_text();
                let config = self.buf.get_config();
                self.cursor = cursor_from_char(&text, self.view.w, &config, start.lc0, 0);
            }
            _ => {
                let (_, cursor) = self.cursor_motion(m, repeat);
                let dx = cursor.c as i32 - self.cursor.c as i32;
                self.remove_range(dx);
            }
        }
        self
    }

    pub fn motion(&mut self, m: &Motion, repeat: usize) -> &mut Self {
        let (_, cursor) = self.cursor_motion(m, repeat);
        self.cursor = cursor;
        //info!("Motion: {:?}", &self.cursor.simple_format());
        self
    }

    pub fn redo(&mut self) -> &mut Self {
        self.buf.redo();
        self
    }

    pub fn undo(&mut self) -> &mut Self {
        self.buf.undo();
        self
    }

    pub fn reset_buffer(&mut self) -> &mut Self {
        let buf = Buffer::from_string(&"".to_string());
        let text = buf.get_text();
        let config = buf.get_config();
        self.start = cursor_start(&text, self.view.w, &config);
        self.cursor = cursor_start(&text, self.view.w, &config);
        self.buf = buf;
        self.clear();
        self
    }

    pub fn command(&mut self, c: &Command) -> &mut Self {
        use Command::*;
        debug!("command {:?}", c);
        match c {
            Insert(x) => self.insert_string(x).update(),
            RemoveChar(dx) => self.remove_range(*dx).update(),
            Undo => self.undo().update(),
            Redo => self.redo().update(),
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn block() {
        // update status
        let view = ViewPos {
            w: 10,
            h: 2,
            x0: 0,
            y0: 0,
        };
        let buf = Buffer::from_string(&"".to_string());
        let mut block = BufferBlock::new(buf, view);
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Insert("x".into()));
        //let commands = block.generate_commands();
        //println!("{:?}", commands);
        //let c = &block.cursor;
        //println!("0:{:?}", c);
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Insert("x\n".into()));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Insert("xyz\n".into()));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Motion(1, Motion::Up));
        block.command(&Command::Motion(1, Motion::Right));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);
        return;

        block.command(&Command::Motion(1, Motion::Left));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Motion(1, Motion::Left));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);

        block.command(&Command::Motion(1, Motion::EOL));
        block.cursor.print();
        log::info!("rc:{:?}", &block.rc);
    }
}
