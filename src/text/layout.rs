use log::*;
use ropey::Rope;
use super::*;
use std::sync::Arc;
use parking_lot::RwLock;
use std::ops::{Deref, DerefMut};
use crossbeam::thread;
use crossbeam::channel;

#[derive(Debug)]
pub struct FileBuffer {
    text: Rope,
    path: String,
    version: u64
}

impl FileBuffer {
    pub fn from_path(path: &String) -> Arc<RwLock<Self>> {
        let text = Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap();
        Arc::new(RwLock::new(FileBuffer { path: path.clone(), text, version: 0 }))
    }

    pub fn from_string(s: &String) -> LockedFileBuffer {
        let text = Rope::from_str(s);
        Arc::new(RwLock::new(FileBuffer { path: "".into(), text, version: 0 }))
    }
}

type LockedFileBuffer = Arc<RwLock<FileBuffer>>;

#[derive(Debug, Clone)]
pub struct BufferWindow {
    status: RenderBlock,
    left: RenderBlock,
    main: RenderBlock,
    buf: LockedFileBuffer,
    cursor: Cursor,
    start: Cursor,
    w: usize, h: usize, x0: usize, y0: usize,
    rc: RenderCursor,
    search_results: SearchResults,
    cache_render_rows: Vec<RowItem>
}

impl BufferWindow {
    fn new(buf: LockedFileBuffer) -> Self {
        let text = buf.read().text.clone();
        Self {
            status: RenderBlock::default(),
            left: RenderBlock::default(),
            main: RenderBlock::default(),
            start: cursor_start(&text, 1),
            cursor: cursor_start(&text, 1),
            w:1, h:0, x0:0, y0:0,
            rc: RenderCursor::default(),
            search_results: SearchResults::default(),
            buf,
            cache_render_rows: vec![]
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.status.clear();
        self.left.clear();
        self.main.clear();
        self.rc.clear();
        self
    }

    pub fn update_from_start(&mut self) -> &mut Self {
        let fb = self.buf.read();
        self.cache_render_rows = LineWorker::screen_from_start(&fb.text, self.main.w, self.main.h, &self.start, &self.cursor);
        let (cx, cy, cursor) = self.locate_cursor_pos_in_window(&self.cache_render_rows);
        info!("start: {:?}", (cx, cy, self.cache_render_rows.len()));
        self.rc.update(cx as usize, cy as usize);
        //self.cx = cx as usize;
        //self.cy = cy as usize;
        self.cursor = cursor;
        drop(fb);
        self
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

    pub fn update(&mut self) -> &mut Self {
        let fb = self.buf.read();

        // render the view
        let (cx, cy, rows) = LineWorker::screen_from_cursor(
            &fb.text, self.main.w, self.main.h, &self.start, &self.cursor);
        // update start based on render
        info!("update: {:?}", (cx, cy, rows.len()));
        let start = rows[0].cursor.clone();
        self.start = start;
        // update cursor position
        self.rc.update(self.main.x0 + cx as usize, self.main.y0 + cy as usize);

        let mut updates = rows.iter().map(|r| {
            let mut u = RowUpdate::default();
            u.item = RowUpdateType::Row(r.clone());
            u
        }).collect::<Vec<RowUpdate>>();
        while updates.len() < self.main.h {
            updates.push(RowUpdate::default());
        }
        self.main.update_rows(updates);

        // update status
        let s = format!(
            "DEBUG: [{},{}] S:{} {} {:?} {:width$}",
            self.rc.cx, self.rc.cy,
            &self.start.simple_format(),
            fb.path,
            (self.main.w, self.main.h, self.main.x0, self.main.y0),
            width=self.status.w);
        self.status.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]);

        // gutter
        let mut gutter = rows.iter().enumerate().map(|(inx, row)| {
            let mut line_display = 0; // zero means leave line blank
            if row.cursor.wrap0 == 0 || inx == 0 {
                line_display = row.cursor.line_inx + 1; // display one based
            }
            let fs;
            if line_display > 0 {
                fs = format!("{:width$}\u{23A5}", line_display, width = self.left.w - 1)
            } else {
                fs = format!("{:width$}\u{23A5}", " ", width=self.left.w - 1)
            }
            RowUpdate::from(LineFormat(LineFormatType::Dim, fs))
        }).collect::<Vec<RowUpdate>>();
        while gutter.len() < self.left.h {
            gutter.push(RowUpdate::default());
        }
        self.left.update_rows(gutter);

        drop(fb);

        // update cache rows
        self.cache_render_rows = rows;
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        out.append(&mut self.status.generate_commands());
        out.append(&mut self.left.generate_commands());
        out.append(&mut self.main.generate_commands());
        out.append(&mut self.rc.generate_commands());
        out
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) -> &mut Self {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;

        self.status.resize(w, 1, x0, y0 + h - 1);
        self.left.resize(6, h - 1, x0, y0);
        self.main.resize(w - 6, h - 1, x0 + 6, y0);
        let text = self.buf.read().text.clone();
        self.cursor = cursor_resize(&text, w, &self.cursor);
        self.start = cursor_resize(&text, w, &self.start);
        self.clear();
        self
    }

    fn remove_char(&mut self) -> &mut Self {
        let mut fb = self.buf.write();
        let c = self.cursor.c;
        if c > 0 {
            fb.text.remove(c-1..c);
            self.cursor = cursor_from_char(&fb.text, self.main.w, c - 1, 0)
                .save_x_hint(self.main.w);
        }
        info!("R: {:?}", (&self.cursor, c));
        drop(fb);
        self
    }

    fn insert_char(&mut self, ch: char) -> &mut Self {
        let mut fb = self.buf.write();
        let c = self.cursor.c;
        fb.text.insert_char(c, ch);
        self.cursor = cursor_from_char(&fb.text, self.main.w, c + 1, 0)
            .save_x_hint(self.main.w);
        info!("I: {:?}", (&self.cursor, c));
        drop(fb);
        self
    }

    pub fn remove_range(&mut self, dx: i32) -> &mut Self {
        let mut fb = self.buf.write();
        self.cursor = cursor_remove_range(&mut fb.text, self.main.w, &self.cursor, dx);
        drop(fb);
        self
    }

    pub fn delete_motion(&mut self, m: &Motion, repeat: usize) -> &mut Self {
        let cursor = self.cursor_motion(m, repeat);
        let dx = cursor.c as i32 - self.cursor.c as i32;
        self.remove_range(dx);
        self
    }

    pub fn motion(&mut self, m: &Motion, repeat: usize) -> &mut Self {
        self.cursor = self.cursor_motion(m, repeat);
        self
    }

    pub fn cursor_move(&mut self, cursor: Cursor) -> &mut Self {
        self.cursor = cursor;
        self
    }

    pub fn search(&mut self, s: &str) -> &mut Self {
        let fb = self.buf.read();
        self.search_results = SearchResults::new_search(&fb.text, s);
        drop(fb);
        self
    }

    pub fn search_next(&mut self, reps: i32) -> &mut Self {
        let fb = self.buf.read();
        let mut cursor = self.cursor.clone();
        cursor = match self.search_results.next_from_position(cursor.c, reps) {
            Some(sub) => {
                cursor_from_char(&fb.text, self.main.w, sub.start(), 0)
            }
            None => cursor
        };
        self.cursor = cursor;
        drop(fb);
        self
    }

    pub fn cursor_motion(&self, m: &Motion, repeat: usize) -> Cursor {
        let text = self.buf.read().text.clone();
        let r = repeat as i32;
        let sx = self.main.w;
        let cursor = &self.cursor;
        match m {
            Motion::Left => cursor_move_to_x(&text, sx, cursor, -r),
            Motion::Right => cursor_move_to_x(&text, sx, cursor, r),
            Motion::Up => cursor_move_to_y(&text, sx, cursor, -r),
            Motion::Down => cursor_move_to_y(&text, sx, cursor, r),
            Motion::BackWord1 => cursor_move_to_word(&text, sx, cursor, -r, false),
            Motion::BackWord2 => cursor_move_to_word(&text, sx, cursor, -r, true),
            Motion::ForwardWord1 => cursor_move_to_word(&text, sx, cursor, r, false),
            Motion::ForwardWord2 => cursor_move_to_word(&text, sx, cursor, r, true),
            Motion::ForwardWordEnd1 => cursor_move_to_word(&text, sx, cursor, r, false),
            Motion::ForwardWordEnd2 => cursor_move_to_word(&text, sx, cursor, r, true),
            Motion::NextSearch => self.search_results.next_cursor(&text, sx, cursor, r),
            Motion::PrevSearch => self.search_results.next_cursor(&text, sx, cursor, -r),
            Motion::Til1(ch) => cursor_move_to_char(&text, sx, cursor, r, *ch, false),
            Motion::Til2(ch) => cursor_move_to_char(&text, sx, cursor, r, *ch, true),
            _ => cursor.clone()
        }
    }

    pub fn cursor_move_line(&mut self, line_inx: i64) -> &mut Self {
        let fb = self.buf.read();
        self.cursor = cursor_from_line_wrapped(&fb.text, self.main.w, line_inx);
        drop(fb);
        self
    }

    pub fn cursor_move_lc(&mut self, dx: i32) -> &mut Self {
        let fb = self.buf.read();
        self.cursor = cursor_move_to_lc(&fb.text, self.main.w, &self.cursor, dx)
            .save_x_hint(self.main.w);
        drop(fb);
        self
    }

    fn cursor_from_xy(&self, mx: usize, my: usize) -> Option<Cursor> {
        let x0 = self.main.x0;
        let y0 = self.main.y0;
        let y1 = y0 + self.main.h;

        let fb = self.buf.read();
        let rows = &self.cache_render_rows;
        if rows.len() > 0 && mx >= x0  && mx < self.main.w && my >= y0 && my < y1 {
            let cx = mx as usize - x0 as usize;
            let cy = my as usize - y0 as usize;
            let mut y = cy;
            if cy >= rows.len() {
                y = rows.len() - 1;
            }
            let mut c = rows[y as usize].cursor.clone();
            c = cursor_to_line_relative(&fb.text, self.main.w, &c, c.wrap0, cx);
            Some(c)
        } else {
            None
        }
    }

    pub fn scroll(&mut self, dy: i32) -> &mut Self {
        let fb = self.buf.read();
        self.start = cursor_move_to_y(&fb.text, self.main.w, &self.start,  dy);
        drop(fb);
        self
    }

}
impl From<LockedFileBuffer> for BufferWindow {
    fn from(item: LockedFileBuffer) -> Self {
        BufferWindow::new(item)
    }
}

pub struct WindowLayout {
    w: usize, h: usize, x0: usize, y0: usize,
    buffers: RotatingList<BufferWindow>
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            w: 10, h: 10, x0: 0, y0: 0,
            buffers: RotatingList::default()
        }
    }
}

impl Deref for WindowLayout {
    type Target = RotatingList<BufferWindow>;
    fn deref(&self) -> &Self::Target {
        &self.buffers
    }
}

impl DerefMut for WindowLayout {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffers
    }
}

impl WindowLayout {
    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        // each buffer needs to be resized on resize event
        // because each one caches things that depend on the size
        self.buffers.elements.iter_mut().for_each(|e| {
            e.resize(w, h, x0, y0);
        });
    }

    fn clear(&mut self) -> &mut Self {
        self.get_mut().clear();
        self
    }
}

pub struct Editor {
    header: RenderBlock,
    command: RenderBlock,
    layout: WindowLayout,
    w: usize, h: usize, x0: usize, y0: usize
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            header: RenderBlock::default(),
            command: RenderBlock::default(),
            layout: WindowLayout::default(),
            w: 10, h: 10, x0: 0, y0: 0
        }
    }
}

impl Editor {
    fn clear(&mut self) -> &mut Self {
        self.header.clear();
        self.command.clear();
        self.layout.clear();
        self
    }

    pub fn update(&mut self) -> &mut Self {
        let b = self.layout.get();
        let fb = b.buf.read();
        let s = format!("Rust-Editor-{} {} {:width$}", clap::crate_version!(), fb.path, b.cursor.simple_format(), width=b.w);
        drop(fb);

        self.header.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]);
        self.command.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Normal, format!("CMD{:width$}", width=b.w)))]);
        self.layout.get_mut().update();

        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = self.layout.get_mut().generate_commands();
        out.append(&mut self.header.generate_commands());
        out.append(&mut self.command.generate_commands());
        out
    }


    fn add_window(&mut self, fb: LockedFileBuffer) {
        let mut bufwin = BufferWindow::from(fb);
        bufwin.resize(self.w, self.h - 2, self.x0, self.y0 + 1);
        self.layout.add(bufwin);
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        self.header.resize(w, 1, x0, y0);
        self.layout.resize(w, h-2, x0, y0 + 1);
        self.command.resize(w, 1, x0, y0 + h - 1);
    }

    pub fn command(&mut self, c: &Command) -> &mut Self {
        use Command::*;
        match c {
            BufferNext => {
                self.layout.next();
                self.layout.get_mut().clear().update();
                let fb = self.layout.buffers.get().buf.read();
                info!("Next: {}", fb.path);
            }
            BufferPrev => {
                self.layout.prev();
                self.layout.get_mut().clear().update();
                let fb = self.layout.get().buf.read();
                info!("Prev: {}", fb.path);
            }
            Insert(x) => {
                self.layout.get_mut().insert_char(*x).update();
            }
            Backspace => {
                self.layout.get_mut().remove_range(-1).update();
            }
            Delete(reps, m) => {
                self.layout.get_mut().delete_motion(m, *reps).update();
            }
            RemoveChar(dx) => {
                self.layout.get_mut().remove_range(*dx).update();
            }
            Motion(reps, m) => {
                self.layout.get_mut().motion(m, *reps).update();
            }
            Search(s) => {
                self.layout.get_mut().search(s.as_str()).search_next(0).update();
            }
            ScrollPage(ratio) => {
                let bw = self.layout.get();
                let xdy = bw.main.w as f32 / *ratio as f32;
                self.layout.get_mut().scroll(xdy as i32).update_from_start();
            }
            Scroll(dy) => {
                self.layout.get_mut().scroll(*dy as i32).update_from_start();
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.layout.get_mut().cursor_move_line(line_inx).update();
            }
            LineNav(dx) => {
                self.layout.get_mut().cursor_move_lc(*dx).update();
            }
            Resize(x, y) => {
                self.resize(*x as usize, *y as usize, self.x0, self.y0);
            }
            Mouse(x, y) => {
                let bw = self.layout.get_mut();
                match bw.cursor_from_xy(*x as usize, *y as usize) {
                    Some(c) => {
                        bw.cursor_move(c);//.update();
                    }
                    _ => ()
                }
            }
            _ => {
                error!("Not implemented: {:?}", c);
            }
        }
        self
    }
}

fn event_loop(editor: &mut Editor) {
    let (tx, rx) = channel::unbounded();

    thread::scope(|s| {
        // user-mode
        s.spawn(|_| {
            let rx = rx.clone();
            let tx = tx.clone();
            main_thread(editor, tx, rx);
        });

        (0..3).for_each(|i| {
            let i = i.clone();
            let rx = rx.clone();
            let tx = tx.clone();
            // save thread
            s.spawn(move |_| {
                info!("background thread {} start", i);
                background_thread(tx, rx);
                info!("background thread {} exit", i);
            });
        });

    }).unwrap();
}

fn main_thread(editor: &mut Editor, tx: channel::Sender<Command>, rx: channel::Receiver<Command>) {
    let mut q = Vec::new();
    let mut mode = Mode::Normal;
    let mut out = std::io::stdout();
    render_reset(&mut out);

    render_commands(&mut out, editor.clear().update().generate_commands());
    render_commands(&mut out, editor.clear().update().generate_commands());

    loop {
        let event = crossterm::event::read().unwrap();

        use std::convert::TryInto;
        let command: Result<Command, _> = event.try_into();
        // see if we got an immediate command
        match command {
            Ok(Command::Quit) => {
                info!("Quit");
                tx.send(Command::Quit).unwrap();
                return;
            }
            Ok(c) => {
                render_commands(&mut out, editor.command(&c).update().generate_commands());
            }
            _ => ()
        }

        // parse user input
        match event.try_into() {
            Ok(e) => {
                q.push(e);
                let result = mode.command()(q.as_slice());
                match result {
                    Ok((_, commands)) => {
                        for c in commands.iter() {
                            match c {
                                Command::Quit => {
                                    info!("Quit");
                                    tx.send(Command::Quit).unwrap();
                                    return;
                                }
                                Command::Mode(m) => {
                                    mode = *m;
                                    q.clear();
                                }
                                _ => {
                                    info!("[{:?}] Ok: {:?}\r", mode, (&q, &c));
                                    q.clear();
                                    render_commands(&mut out, editor.command(&c).update().generate_commands());
                                }
                            }
                        }
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

fn background_thread(tx: channel::Sender<Command>, rx: channel::Receiver<Command>) {
    loop {
        channel::select! {
            recv(rx) -> c => {
                match c {
                    Ok(Command::SaveBuffer(path, text)) => {
                        Buffer::save_text(&path, &text);
                    }
                    Ok(Command::Quit) => {
                        tx.send(Command::Quit).unwrap();
                        break;
                    }
                    Ok(c) => {
                        info!("C: {:?}", (c));
                    }
                    Err(e) => {
                        info!("Error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
}

pub fn layout_test() {
    let params = crate::cli::get_params();
    let mut e = Editor::default();

    let fb1 = FileBuffer::from_path(&String::from("asdf.txt"));
    let fb2 = FileBuffer::from_path(&String::from("asdf2.txt"));
    e.add_window(fb1.clone());
    e.add_window(fb2.clone());
    e.add_window(fb2.clone());

    if params.paths.len() == 0 {
        let fb = FileBuffer::from_string(&"".into());
        e.add_window(fb.clone());
    }

    use std::path::Path;
    params.paths.iter().for_each(|path| {
        if Path::new(&path).exists() {
            let fb = FileBuffer::from_path(&path.clone());
            e.add_window(fb.clone());
        }
    });

    //e.resize(100,20,0,0);

    //use Command::*;
    //let cs = vec![Insert('x'), BufferNext, Insert('y'), BufferNext, Insert('z')];
    //cs.iter().for_each(|c| e.command(c));
    //info!("A: {:?}", &fb1);
    //info!("B: {:?}", &fb2);
    //info!("C: {:?}", &mut e.layout.get_mut().generate_commands());

    use crossterm::{execute};
    use crossterm::terminal;
    use crossterm::event;
    let mut out = std::io::stdout();
    terminal::enable_raw_mode().unwrap();
    execute!(out, event::EnableMouseCapture).unwrap();
    execute!(out, terminal::DisableLineWrap).unwrap();

    let (sx, sy) = crossterm::terminal::size().unwrap();
    e.resize(sx as usize, sy as usize, 0, 0);
    info!("terminal: {:?}", (sx, sy));
    info!("paths: {:?}", (params.paths));
    //event_loop(params.paths, sx as usize, sy as usize);
    event_loop(&mut e);
    execute!(out, event::DisableMouseCapture).unwrap();
    execute!(out, terminal::EnableLineWrap).unwrap();
    terminal::disable_raw_mode().unwrap();
}


