use log::*;
use super::*;

#[derive(Debug)]
pub struct ViewCursor {
    cx: u16,
    cy: u16,
    _x_hint: u16,
    _has_changed: bool
}
impl ViewCursor {
    fn new() -> Self {
        Self { cx: 0, cy: 0, _has_changed: false, _x_hint: 0 }
    }

    pub fn set_x_hint(&mut self, x: u16) {
        self._x_hint = x;
    }

    pub fn x_hint(&self) -> u16 {
        self._x_hint
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
    linewrap: LineWrap<'a>,
    // viewport start/cursor/end
    pub port: ViewPort,
    pub mode: Mode,
    pub spec: ViewSpec,
    lines: Vec<ViewRow>,
    pub cursor: ViewCursor,
    header: ViewRow,
    status: ViewRow,
    command: ViewRow
}

impl<'a> BufferView<'a> {
    pub fn new(buf: &'a mut SmartBuffer<'a>, mode: Mode, spec: ViewSpec) -> Self {
        Self {
            buf: buf,
            linewrap: LineWrap::default(),
            port: ViewPort::default(),
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

    pub fn char_to_wrap(&self, c: usize) -> Option<Wrap> {
        self.buf.char_to_wrap(self.spec.sx, c)
    }

    fn prev_wrap(&self, w: &Wrap) -> Option<Wrap> {
        self.buf.prev_wrap(self.spec.sx, w)
    }

    fn next_wrap(&self, w:  &Wrap) -> Option<Wrap> {
        self.buf.next_wrap(self.spec.sx, w)
    }

    fn delta_wrap(&self, c: usize, dy: i32) -> Wrap {
        self.buf.delta_wrap(self.spec.sx, c, dy)
    }

    fn wrap_window_down(&self, c: usize, size: usize) -> Vec<Wrap> {
        self.wrap_window(c, size, false)
    }

    fn wrap_window(&self, c: usize, size: usize, reverse: bool) -> Vec<Wrap> {
        self.buf.wrap_window(self.spec.sx, c, size, reverse)
    }

    pub fn line_move(&self, x: i32) -> usize {
        self.buf.line_move(self.spec.sx, self.port.char_current, x)
    }

    pub fn move_cursor_x(&mut self, c0: usize, dx: i32) -> (usize, usize) {
        self.buf.move_cursor_x(self.spec.sx, c0, dx)
    }

    pub fn move_cursor_y(&mut self, c0: usize, dy: i32) -> usize {
        self.buf.move_cursor_y(self.spec.sx, c0, &self.cursor, dy)
    }

    pub fn jump_to_line(&mut self, line: i64) -> usize {
        self.buf.jump_to_line(line)
    }

    pub fn scroll(&mut self, y: i32) {
        let w = self.delta_wrap(self.port.char_start, y);
        if w.c0 != self.port.char_start {
            self.port.char_start = w.c0;
            self.update_lines();
            let first = self.lines.get(0).unwrap();
            let last = self.lines.get(self.lines.len()-1).unwrap();

            // start and end of view port
            self.port.char_start = first.c0;
            self.port.char_end = last.c1;

            let mut c = self.port.char_current;
            let start = first.c0;
            if c < start {
                c = start;
            } else if c >= self.port.char_end {
                c = last.c0;
            }
            self.update_cursor(c);
        }
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

    pub fn cursor_from_char(&self, c: usize) -> Option<(u16, u16)> {
        // find and set cursor
        match self.lines.iter().position(|w| w.is_within(c)) {
            Some(inx) => {
                let w = &self.lines[inx];
                let s = self.buf.to_string(w.c0, c, true);
                //let cx = (c - w.c0) as u16;
                let cx = s.len() as u16;
                let cy = inx as u16;
                Some((cx, cy))
            }
            None => None
        }
    }

    pub fn update_cursor(&mut self, c: usize) {
        self.port.char_current = c;
        // find and set cursor
        if let Some((cx, cy)) = self.cursor_from_char(c) {
            self.cursor.update(cx, cy);
        }
    }

    pub fn update_lines(&mut self) {
        let c = self.port.char_start;
        let sy = self.spec.sy as usize;
        let wraps = self.wrap_window_down(c, sy);
        let mut inx = 0;
        while inx < sy {
            //info!("X:{:?}", (inx, self.lines.len(), &self.lines));
            let line = self.lines.get_mut(inx).unwrap();
            match wraps.get(inx) {
                Some(w) => {
                    line.update_string(self.buf.wrap_to_string(&w, true));
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
            out.push(DrawCommand::Status(row, format!("Header: {:?}", self.port.char_start).into()));
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
            out.push(DrawCommand::Status(row, format!("DEBUG: {:?}", self.port.char_start).into()));
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
                self.buf.insert_char(self.port.char_current, c);
                self.port.char_current += 1;
                self.update_lines();
                self.update_cursor(self.port.char_current);
            }
            Command::Refresh => {
                self.refresh();
            }
            _ => {}
        }
    }
}



