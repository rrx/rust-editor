use log::*;
use std::io;
use std::fs::File;
use ropey::iter::{Bytes, Chars, Chunks, Lines};
use ropey::{Rope, RopeSlice};
use crate::frontend::DrawCommand;
use crate::ism::{Mode, Command};
use crate::text::wrap::WrapValue;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::convert::TryInto;

#[derive(Debug)]
pub struct SmartBuffer<'a> {
    text: Rope,
    path: &'a str,
    dirty: bool
}

impl<'a> SmartBuffer<'a> {
    pub fn new(text: Rope, path: &'a str) -> Self {
        Self {
            text: text,
            path: path,
            dirty: false
        }
    }

    pub fn from_path(path: &'a str) -> io::Result<Self> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path)?))?;
        Ok(Self::new(text, path))
    }

    pub fn from_str(s: &str) -> Self {
        let text = Rope::from_str(s);
        Self::new(text, "")
    }

    pub fn insert_char(&mut self, c: usize, ch: char) {
        self.dirty = true;
        self.text.insert_char(c, ch);
    }


}

#[derive(Debug)]
pub enum RowType {
    Line(String),
    EOF
}

#[derive(Debug)]
struct ViewRow {
    body: RowType,
    checksum: u64,
    dirty: bool
}

impl ViewRow {
    fn new(body: String) -> Self {
        Self { body: RowType::Line(body), checksum: 0, dirty: false }.init()
    }

    fn init(mut self) -> Self {
        self.update_hash();
        self.dirty = true;
        self
    }

    fn update_hash(&mut self) -> bool {
        let mut h = DefaultHasher::new();
        let v = match &self.body {
            RowType::Line(s) => {
                s.hash(&mut h);
                h.finish()
            }
            EOF => 0
        };
        let changed = v != self.checksum;
        self.checksum = v;
        changed
    }

    fn make_eof(&mut self) {
        self.body = RowType::EOF;
        self.dirty = self.update_hash();
    }

    fn update(&mut self, body: String) {
        self.body = RowType::Line(body);
        self.dirty = self.update_hash();
    }

    fn clear(&mut self) {
        self.dirty = false;
    }
}

impl Default for ViewRow {
    fn default() -> Self {
        Self::new("".into())
    }
}

use std::cmp::{Eq, PartialEq};
impl PartialEq for ViewRow {
    fn eq(&self, other: &Self) -> bool {
        self.checksum == other.checksum
    }
}
impl Eq for ViewRow {}

#[derive(Debug)]
pub struct BufferView<'a> {
    buf: &'a mut SmartBuffer<'a>,
    // viewport start/cursor/end
    char_start: usize,
    char_current: usize,
    char_end: usize,
    mode: Mode,
    spec: ViewSpec,
    lines: Vec<ViewRow>
}

impl<'a> BufferView<'a> {
    fn new(buf: &'a mut SmartBuffer<'a>, mode: Mode, spec: ViewSpec) -> Self {
        Self {
            buf: buf,
            char_start: 0,
            char_current: 0,
            char_end: 0,
            mode: Mode::Normal,
            spec: spec,
            lines: Vec::new()
        }.init()
    }

    fn init(mut self) -> Self {
        self.lines.resize_with(self.spec.sy as usize, ViewRow::default);
        self
    }

    fn char_to_wrap(&self, c: usize) -> Option<WrapValue> {
        let text = &self.buf.text;
        let len_chars = text.len_chars();
        if c >= len_chars && len_chars > 0 {
            self.char_to_wrap(len_chars-1)
        } else {
            let vsx = self.spec.sx as usize;
            let line = text.char_to_line(c);
            let lc0 = text.line_to_char(line);
            let lc1 = text.line_to_char(line+1);
            let wrap0 = (c - lc0) / vsx;
            let c0 = lc0 + wrap0 * vsx;
            let mut wrap1 = wrap0 + 1;
            let wraps = (lc1 - lc0) / vsx + 1;
            let c1;
            if wrap1 == wraps {
                c1 = lc1;
                wrap1 = 0;
            } else {
                c1 = c0 + vsx;
            }
            Some(WrapValue {
                lc0: lc0,
                lc1: lc1,
                c0: c0,
                c1: c1,
                offset: c - c0,
                wrap0: wrap0,
                wrap1: wrap1,
                line0: line,
                line1: line+1,
                wraps: wraps,
                dirty: true
            })
        }
    }

    fn prev_wrap(&self, w: &WrapValue) -> Option<WrapValue> {
        let vsx = self.spec.sx as usize;
        if w.wrap0 > 0 {
            let c0 = w.lc0 + (w.wrap0-1) * vsx;
            self.char_to_wrap(c0)
        } else if w.line0 > 0 {
            let offset = w.offset;
            let nw = self.char_to_wrap(w.lc0-1);
            if let Some(mut w0) = nw {
                w0.offset = offset;
                Some(w0)
            } else {
                nw
            }
        } else {
            None
        }
    }

    fn next_wrap(&self, w:  &WrapValue) -> Option<WrapValue> {
        let len_chars = self.buf.text.len_chars();
        if w.c1 >= len_chars {
            None
        } else {
            self.char_to_wrap(w.c1)
        }
    }

    fn delta_wrap(&self, c: usize, dy: i32) -> WrapValue {
        let start = c;
        let mut w = self.char_to_wrap(start).unwrap();

        if dy > 0 {
            let mut count = dy;
            while count > 0 {
                match self.next_wrap(&w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }

        if dy < 0 {
            let mut count = (-dy) as usize;
            while count > 0 {
                match self.prev_wrap(&w) {
                    Some(x) => {
                        w = x;
                        count -= 1;
                    }
                    _ => break
                }
            }
        }
        w
    }

    fn wrap_window_down(&self, c: usize, size: usize) -> Vec<WrapValue> {
        self.wrap_window(c, size, false)
    }

    fn wrap_window(&self, c: usize, size: usize, reverse: bool) -> Vec<WrapValue> {
        let mut out = Vec::new();
        let ow = self.char_to_wrap(c);

        let r;
        if reverse {
            r = -1;
        } else {
            r = 1;
        }

        if ow.is_some() && size > 0 {
            let mut w = ow.unwrap();
            out.push(w);

            let mut count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(c, r*count);
                if w0.c0 == w.c0 {
                    break;
                }
                out.push(w0);
                w = w0;
                count += 1;
            }

            w = ow.unwrap();
            count = 1;
            while out.len() < size {
                let w0 = self.delta_wrap(c, -r*count);
                if w0.c0 == w.c0 {
                    break;
                }
                out.insert(0,w0);
                w = w0;
                count += 1;
            }
        }
        if reverse {
            out.reverse();
        }
        out
    }

    pub fn update_lines(&mut self) {
        let c = self.char_start;
        let sy = self.spec.sy as usize;
        let wraps = self.wrap_window_down(c, sy);
        let mut inx = 0;
        while inx < sy {
            //info!("X:{:?}", (inx, self.lines.len(), &self.lines));
            let line = self.lines.get_mut(inx).unwrap();
            match wraps.get(inx) {
                Some(w) => {
                    line.update(wrap_to_string(&w, &self.buf.text));
                },
                None => {
                    line.update("".into())
                }
            }
            inx += 1;
        }
    }

    // try to only render the lines that have changed
    pub fn render(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        for (inx, line) in self.lines.iter_mut().enumerate() {
            if line.dirty {
                let s = match &line.body {
                    RowType::Line(x) => String::from(x),
                    _ => "".into()
                }.replace("\n", ".");
                out.push(DrawCommand::Row(self.spec.x0, inx as u16 + self.spec.y0, s.clone()));
                line.clear();
            }
        }

        let mut row = self.spec.y0 + self.spec.sy;
        if self.spec.status > 0 {
            out.push(DrawCommand::Status(row, "DEBUG".into()));
            row += 1;
        }
        if self.spec.footer > 0 {
            out.push(DrawCommand::Status(row, "".to_string()));
        }
        out
    }

    fn refresh(&mut self) {
        for (inx, line) in self.lines.iter_mut().enumerate() {
            line.dirty = true;
        }
    }

    fn command(&mut self, command: Command) {
        info!("Command: {:?}", command);
        match command {
            Command::Insert(c) => {
                self.buf.text.insert_char(self.char_current, c);
            }
            Command::Refresh => {
                self.refresh();
            }
            _ => {}
        }
    }
}

pub fn wrap_to_string<'a>(w: &WrapValue, text: &Rope) -> String {
    text.slice(w.c0..w.c1).to_string()
}


#[derive(Debug)]
pub struct ViewSpec {
    x: u16,
    y: u16,
    origin_x: u16,
    origin_y: u16,
    header: u16, // header rows
    footer: u16, // footer rows
    status: u16, // status rows
    lm: u16, // left margin
    rm: u16, // right margin
    sx: u16, // horizontal size for body
    sy: u16, // vertical size for body
    x0: u16, // x origin for body
    y0: u16, // y origin for body
}

impl ViewSpec {
    fn new(x: u16, y: u16, origin_x: u16, origin_y: u16) -> Self {
        let header = 0;
        let footer = 1;
        let status = 1;
        let lm = 5;
        let rm = 1;
        let s = Self {
            x: x,
            y: y,
            origin_x: origin_x,
            origin_y: origin_y,
            header: header,
            footer: footer,
            status: status,
            lm: lm,
            rm: rm,
            sx: 0,
            sy: 0,
            x0: 0,
            y0: 0
        };
        s.init()
    }

    fn init(mut self) -> Self {
        self.calc();
        self
    }

    fn update(&mut self, x: u16, y: u16, origin_x: u16, origin_y: u16) {
        self.x = x;
        self.y = y;
        self.origin_x = origin_x;
        self.origin_y = origin_y;
        self.calc();
    }

    fn calc(&mut self) {
        self.sx = self.x - self.lm - self.rm;
        self.sy = self.y - self.header - self.footer - self.status;
        self.x0 = self.origin_x + self.lm;
        self.y0 = self.origin_y + self.header;
    }

}


#[derive(Debug)]
pub struct App<'a> {
    view: BufferView<'a>
}

use crate::ism::FrontendTrait;

impl<'a> App<'a> {
    pub fn new(buf: &'a mut SmartBuffer<'a>, x: u16, y: u16) -> Self {
        let spec = ViewSpec::new(x, y, 0, 0);
        let mut s = Self {
            view: BufferView::new(buf, Mode::Normal, spec)
        };
        s.update(x, y, 0, 0);
        s
    }

    fn update(&mut self, x: u16, y: u16, origin_x: u16, origin_y: u16) {
        self.view.spec.update(x, y, origin_x, origin_y);
        self.view.update_lines();
        self.view.refresh();
    }

    fn test(&mut self) {
        self.view.spec.rm += 1;
        self.view.spec.calc();
        self.view.update_lines();
        self.view.refresh();
        info!("T: {:?}", (self.view.spec));
    }

    fn command(&mut self, command: Command) {
        info!("Command: {:?}", command);
        match command {
            Command::Mode(m) => {
                self.view.mode = m;
            }
            Command::Test => {
                self.test()
            }
            Command::MoveCursorX(dx) => {
                //self.move_cursor_x(self.char_current, dx);
            }
            Command::MoveCursorY(dy) => {
                //self.move_cursor_y(self.char_current, dy);
            }
            Command::ScrollPage(dy) => {
                //let xdy = self.view.vsy as f32 / dy as f32;
                //self.scroll(xdy as i32);
            }
            Command::Scroll(dy) => {
                //self.scroll(dy as i32);
            }

            Command::LineNav(x) => {
                //self.line_move(x);
            }

            // Goto a line
            Command::Line(line) => {
                //self.scroll_line(line);
            }

            Command::Resize(x, y) => {
                self.update(x, y, 0, 0);
            }

            Command::Mouse(x, y) => {
                //if x >= 6 && y < self.view.vsy {
                    //let mut cx = x as usize - 6;
                    //let cy = y as usize;
                    //let w = self.view.wraps[cy];
                    //let line_length = w.c1 - w.c0;
                    //if cx >= line_length {
                        //cx = line_length - 1;
                    //}
                    //let c = w.c0 + cx;
                    //self.update_window(c);
                //}
            }
            _ => self.view.command(command)
        }
    }

    pub fn process(&mut self, fe: &mut dyn FrontendTrait) {
        let mut q = Vec::new();
        fe.reset();
        fe.render(self.view.render());
        loop {
            let event = crossterm::event::read().unwrap();

            // see if we got a command
            match event.try_into() {
                Ok(Command::Quit) => {
                    info!("Quit");
                    return;
                }
                Ok(c) => {
                    self.command(c);
                    fe.render(self.view.render());
                    continue;
                }
                _ => ()
            }

            // run parse otherwise
            match event.try_into() {
                Ok(e) => {
                    q.push(e);
                    let result = self.view.mode.command()(q.as_slice());
                    match result {
                        Ok((_, Command::Quit)) => {
                            info!("Quit");
                            return;
                        }
                        Ok((_, x)) => {
                            info!("[{:?}] Ok: {:?}\r", &self.view.mode, (&q, &x));
                            q.clear();
                            self.command(x);
                            fe.render(self.view.render());
                        }
                        Err(nom::Err::Incomplete(_)) => {
                            info!("Incomplete: {:?}\r", (q));
                        }
                        Err(e) => {
                            info!("Error: {:?}\r", (e, &q));
                            q.clear();
                        }
                    }
                }
                Err(err) => {
                    info!("ERR: {:?}\r", (err));
                }
            }
        }
    }

}

pub fn app_debug(filepath: &str) {
    let mut fe = crate::frontend_debug::FrontendDebug::new();
    let mut buf = SmartBuffer::from_path(filepath).unwrap();
    let mut app = App::new(&mut buf, 20, 10);
    app.process(&mut fe);
}

pub fn raw(filepath: &str) {
    use crossterm::*;
    use crossterm::terminal::*;
    use crossterm::event::*;
    // set initial size
    let mut fe = crate::frontend_crossterm::FrontendCrossterm::new();
    let (sx, sy) = terminal::size().unwrap();
    let mut buf = SmartBuffer::from_path(filepath).unwrap();
    let mut app = App::new(&mut buf, sx, sy);
    let mut out = std::io::stdout();
    enable_raw_mode().unwrap();
    execute!(out, EnableMouseCapture).unwrap();
    app.process(&mut fe);
    execute!(out, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
}

