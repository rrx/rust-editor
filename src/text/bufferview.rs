use log::*;
use super::*;

#[derive(Debug)]
pub struct ViewCursor {
    cx: u16,
    cy: u16,
    _has_changed: bool
}
impl ViewCursor {
    fn new() -> Self {
        Self { cx: 0, cy: 0, _has_changed: false }
    }

    fn update(&mut self, cx: u16, cy: u16) {
        if cx != self.cx { self._has_changed = true }
        if cy != self.cy { self._has_changed = true }
        self.cx = cx;
        self.cy = cy;
    }

    fn clear(&mut self) {
        self._has_changed = false;
    }

    fn pos(&self) -> (u16, u16) {
        (self.cx, self.cy)
    }

    fn has_changed(&self) -> bool {
        self._has_changed
    }
}

#[derive(Debug)]
pub struct BufferView<'a> {
    buf: &'a mut SmartBuffer<'a>,
    // viewport start/cursor/end
    char_start: usize,
    pub char_current: usize,
    char_end: usize,
    pub mode: Mode,
    pub spec: ViewSpec,
    lines: Vec<ViewRow>,
    cursor: ViewCursor,
    header: ViewRow,
    status: ViewRow,
    command: ViewRow
}

impl<'a> BufferView<'a> {
    pub fn new(buf: &'a mut SmartBuffer<'a>, mode: Mode, spec: ViewSpec) -> Self {
        Self {
            buf: buf,
            char_start: 0,
            char_current: 0,
            char_end: 0,
            mode: Mode::Normal,
            spec: spec,
            lines: Vec::new(),
            cursor: ViewCursor::new(),
            header: ViewRow::default(),
            status: ViewRow::default(),
            command: ViewRow::default()
        }.init()
    }

    fn init(mut self) -> Self {
        self.resize(self.spec.w, self.spec.h, self.spec.origin_x, self.spec.origin_y);
        //self.lines.resize_with(self.spec.sy as usize, ViewRow::default);
        self
    }

    pub fn resize(&mut self, w: u16, h: u16, origin_x: u16, origin_y: u16) {
        self.spec.resize(w, h, origin_x, origin_y);
        self.lines.resize_with(self.spec.sy as usize, ViewRow::default);
    }

    fn char_to_wrap(&self, c: usize) -> Option<WrapValue> {
        self.buf.char_to_wrap(c, self.spec.sx as usize)
    }

    fn prev_wrap(&self, w: &WrapValue) -> Option<WrapValue> {
        self.buf.prev_wrap(w, self.spec.sx as usize)
    }

    fn next_wrap(&self, w:  &WrapValue) -> Option<WrapValue> {
        self.buf.next_wrap(w, self.spec.sx as usize)
    }

    fn delta_wrap(&self, c: usize, dy: i32) -> WrapValue {
        self.buf.delta_wrap(c, dy, self.spec.sx as usize)
    }

    fn wrap_window_down(&self, c: usize, size: usize) -> Vec<WrapValue> {
        self.wrap_window(c, size, false)
    }

    fn wrap_window(&self, c: usize, size: usize, reverse: bool) -> Vec<WrapValue> {
        self.buf.wrap_window(c, size, reverse, self.spec.sx as usize)
    }

    pub fn line_move(&self, x: i32) -> usize {
        self.buf.line_move(self.char_current, self.spec.sx as usize, x)
    }

    pub fn char_from_cursor(&self, mx: u16, my: u16) -> Option<usize> {
        let ViewSpec { x0, y0, sx, sy, ..} = self.spec;
        let x1 = x0 + sx;
        let y1 = y0 + sy;
        if mx >= x0  && mx < sx && my >= y0 && my < y1 {
            let mut cx = mx as usize - x0 as usize;
            let cy = my as usize - y0 as usize;
            let line = self.lines.get(cy).unwrap();
            if line.is_line() {
                let line_length = line.c1 - line.c0;
                if cx >= line_length {
                    cx = line_length - 1;
                }
                let c = line.c0 + cx;
                info!("C: {:?}", (cx,cy, c, mx, my, line_length, x1, y1, line, &self.spec));
                Some(c)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cursor_from_char(&self, c: usize) -> (u16, u16) {
        // find and set cursor
        let inx = self.lines.iter().position(|w| w.is_within(c)).unwrap();
        let w = &self.lines[inx];
        let cx = (c - w.c0) as u16;
        let cy = inx as u16;
        (cx, cy)
    }

    pub fn update_cursor(&mut self, c: usize) {
        self.char_current = c;
        // find and set cursor
        let (cx, cy) = self.cursor_from_char(c);
        self.cursor.update(cx, cy);
        //self.cx = cx;
        //self.cy = cy;
        //self.set_cursor(cx as u16,cy);
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
                    line.update_string(self.buf.wrap_to_string(&w));
                    line.update_wrap(&w);
                },
                None => {
                    line.update_string("".into())
                }
            }
            inx += 1;
        }
    }

    // try to only render the lines that have changed
    pub fn render(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        let mut row = self.spec.origin_y;
        if self.spec.header > 0 {
            out.push(DrawCommand::Status(row, format!("Header: {:?}", self.char_start).into()));
            row += self.spec.header;
        }
        for line in self.lines.iter_mut() {
            if line.dirty {
                let s = line.to_string();
                out.push(DrawCommand::Row(self.spec.x0, row, s.clone()));
                line.clear();
            }
            row += 1;
        }

        if self.spec.status > 0 {
            out.push(DrawCommand::Status(row, format!("DEBUG: {:?}", self.char_start).into()));
            row += self.spec.status;
        }
        if self.spec.footer > 0 {
            out.push(DrawCommand::Status(row, "".to_string()));
        }

        //if self.cursor.has_changed() {
            //self.cursor.clear();
            let (cx, cy) = self.cursor.pos();
            out.push(DrawCommand::Cursor(cx + self.spec.x0, cy + self.spec.y0));
        //}
        out
    }

    pub fn refresh(&mut self) {
        for line in self.lines.iter_mut() {
            line.dirty = true;
        }
    }

    pub fn command(&mut self, command: Command) {
        info!("Command: {:?}", command);
        match command {
            Command::Insert(c) => {
                self.buf.insert_char(self.char_current, c);
                self.char_current += 1;
                self.update_lines();
                self.update_cursor(self.char_current);
            }
            Command::Refresh => {
                self.refresh();
            }
            _ => {}
        }
    }
}



