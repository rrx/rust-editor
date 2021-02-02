use log::*;
use ropey::Rope;
use super::*;
use std::sync::Arc;
use parking_lot::RwLock;
use std::ops::{Deref, DerefMut};
use crossbeam::thread;
use crossbeam::channel;
use signal_hook::low_level;

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
        info!("buffer start: {:?}", (cx, cy, self.cache_render_rows.len()));
        self.rc.update(cx as usize, cy as usize);
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

        // refresh the cursors, which might contain stale data
        self.start = cursor_update(&fb.text, self.main.w, &self.start);
        self.cursor = cursor_update(&fb.text, self.main.w, &self.cursor);

        // render the view, so we know how long the line is on screen
        let (cx, cy, rows) = LineWorker::screen_from_cursor(
            &fb.text, self.main.w, self.main.h, &self.start, &self.cursor);
        // update start based on render
        info!("buffer update: {:?}", (cx, cy, rows.len()));
        let start = rows[0].cursor.clone();
        self.start = start;
        // update cursor position
        self.rc.update(self.main.x0 + cx as usize, self.main.y0 + cy as usize);

        // generate updates
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
            "DEBUG: [{},{}] S:{} {} {:?}{:width$}",
            self.rc.cx, self.rc.cy,
            &self.start.simple_format(),
            fb.path,
            (self.main.w, self.main.h, self.main.x0, self.main.y0),
            "",
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
        info!("remove: {:?}", (&self.cursor, c));
        drop(fb);
        self
    }

    fn insert_char(&mut self, ch: char) -> &mut Self {
        let mut fb = self.buf.write();
        let c = self.cursor.c;
        fb.text.insert_char(c, ch);
        self.cursor = cursor_from_char(&fb.text, self.main.w, c + 1, 0)
            .save_x_hint(self.main.w);
        info!("insert: {:?}", (&self.cursor, c));
        drop(fb);
        self
    }

    pub fn remove_range(&mut self, dx: i32) -> &mut Self {
        let mut fb = self.buf.write();
        info!("remove range: {:?}", (&self.cursor, dx));
        self.cursor = cursor_remove_range(&mut fb.text, self.main.w, &self.cursor, dx);
        drop(fb);
        self
    }

    pub fn delete_motion(&mut self, m: &Motion, repeat: usize) -> &mut Self {
        match m {
            Motion::Line => {
                let mut fb = self.buf.write();
                let mut c = self.cursor.clone();
                (0..repeat).for_each(|_| {
                    c = cursor_delete_line(&mut fb.text, self.main.w, &c);
                });
                self.cursor = c;
                drop(fb);
            },
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

    pub fn paste_motion(&mut self, m: &Motion, s: &String, reps: usize) -> &mut Self {
        let (_, c) = self.cursor_motion(m, 1);
        let mut fb = self.buf.write();
        (0..reps).for_each(|_| fb.text.insert(c.c, s.as_str()));
        drop(fb);
        self
    }

    pub fn motion_slice(&mut self, m: &Motion) -> String {
        let c1 = self.cursor.c;
        let c2 = self.cursor_motion(m, 1).1.c;
        let r = if c1 > c2 {
            c2..c1
        } else {
            c1..c2
        };
        self.buf.read().text.slice(r).to_string()
    }

    pub fn cursor_motion(&self, m: &Motion, repeat: usize) -> (Cursor, Cursor) {
        let text = self.buf.read().text.clone();
        let r = repeat as i32;
        let sx = self.main.w;
        let cursor = &self.cursor;
        use Motion::*;
        let c1 = cursor.clone();
        let c2 = cursor.clone();
        match m {
            OnCursor => (c1, c2),
            AfterCursor => (c1, cursor_move_to_x(&text, sx, cursor, 1)),
            Line => {
                let line0 = cursor.line_inx;
                let line1 = cursor.line_inx + 1;
                (
                    cursor_from_line(&text, sx, line0),
                    cursor_from_line(&text, sx, line1),
                )
            }
            EOL => (c1, cursor_move_to_lc(&text, sx, cursor, -1)),
            NextLine => (c1, cursor_from_line(&text, sx, cursor.line_inx + 1)),
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
            _ => (c1, c2)
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

use std::collections::HashMap;
pub struct Registers {
    regs: HashMap<Register,String>
}
impl Default for Registers {
    fn default() -> Self {
        Self { regs: HashMap::new() }
    }
}
impl Registers {
    fn update(&mut self, r: &Register, s: &String) {
        info!("Reg[{:?}] = {}", r, &s);
        self.regs.insert(*r, s.clone());
    }
    fn get(&self, r: &Register) -> String {
        self.regs.get(r).unwrap_or(&String::from("")).clone()
    }
}

pub struct Editor {
    header: RenderBlock,
    command: RenderBlock,
    layout: WindowLayout,
    registers: Registers,
    highlight: String,
    w: usize, h: usize, x0: usize, y0: usize,
    terminal: Terminal
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            header: RenderBlock::default(),
            command: RenderBlock::default(),
            layout: WindowLayout::default(),
            registers: Registers::default(),
            highlight: String::new(),
            w: 10, h: 10, x0: 0, y0: 0,
            terminal: Terminal::default()
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
        self.command.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Normal, format!("CMD{:width$}", "", width=b.w)))]);
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

    pub fn start_terminal(&mut self) {
        info!("enter raw terminal");
        self.terminal.enter_raw_mode();
        //self.in_terminal = true;
    }

    pub fn exit_terminal(&mut self) {
        info!("exit raw terminal");
        self.terminal.leave_raw_mode();
        //self.in_terminal = false;
    }

    //pub fn toggle_terminal(&mut self) {
        //self.terminal.toggle();
        //if self.in_terminal {
            //self.exit_terminal();
        //} else {
            //self.start_terminal();
        //}
    //}

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        info!("Resize: {}/{}", w, h);
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        self.header.resize(w, 1, x0, y0);
        self.layout.resize(w, h-2, x0, y0 + 1);
        self.command.resize(w, 1, x0, y0 + h - 1);
    }

    pub fn command(&mut self, c: &Command) -> &mut Self {
        use crate::bindings::parser::Motion as M;
        use Command::*;
        match c {
            BufferNext => {
                self.layout.next().get_mut().clear().update();
                self.layout.get_mut().main.set_highlight(self.highlight.clone());
                let fb = self.layout.buffers.get().buf.read();
                info!("Next: {}", fb.path);
            }
            BufferPrev => {
                self.layout.prev().get_mut().clear().update();
                self.layout.get_mut().main.set_highlight(self.highlight.clone());
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
            Yank(reg, m) => {
                self.registers.update(reg, &self.layout.get_mut().motion_slice(m));
                self.update();
            }
            Paste(reps, reg, m) => {
                let s = self.registers.get(reg);
                self.layout.get_mut().paste_motion(m, &s, *reps).update();
            }
            RemoveChar(dx) => {
                self.layout.get_mut().remove_range(*dx).update();
            }
            Motion(reps, m) => {
                self.layout.get_mut().motion(m, *reps).update();
            }
            SearchInc(s) => {
                self.highlight = s.clone();
                self.layout.get_mut().main.set_highlight(s.clone());
            }
            Search(s) => {
                self.highlight = s.clone();
                self.layout.get_mut().search(s.as_str()).search_next(0).update();
                self.layout.get_mut().main.set_highlight(s.clone());
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

            Quit => {
                info!("Quit");
                self.terminal.cleanup();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGHUP).unwrap();
            }

            Refresh => {
                info!("Refresh");
                self.terminal.enter_raw_mode();
                let (sx, sy) = crossterm::terminal::size().unwrap();
                self.resize(sx as usize, sy as usize, 0, 0);
                self.clear().update();
            }

            Resume => {
                info!("Resume");
                self.terminal.enter_raw_mode();
                let (sx, sy) = crossterm::terminal::size().unwrap();
                self.resize(sx as usize, sy as usize, 0, 0);
                self.clear().update();
            }

            Stop => {
                info!("Stop");
                self.terminal.leave_raw_mode();
                //use std::{io::stdout, time::Duration};
                //use nix::sys::signal;
                //use libc;

                //std::thread::sleep(std::time::Duration::from_millis(1000));
                //Duration
                //self.terminal.toggle();
                //self.toggle_terminal();
                //let mut out = std::io::stdout();
                //if self.in_terminal {
                    //execute!(out, terminal::LeaveAlternateScreen).unwrap();
                    //println!("{}", char::from_u32(0x001a).unwrap());
                signal_hook::low_level::raise(signal_hook::consts::signal::SIGSTOP).unwrap();
                    //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                    //signal_hook::low_level::raise(signal_hook::consts::signal::SIGSTOP).unwrap();
                    //low_level::emulate_default_handler(SIGSTOP).unwrap();
                //} else {
                    //execute!(out, terminal::EnterAlternateScreen).unwrap();
                    //self.clear().update();
                //}
                //self.in_terminal = !self.in_terminal;
                //terminal_cleanup();
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                //self.command(&Command::Resume);
                //signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
                //println!("{}", char::from_u32(0x001a).unwrap());
                //low_level::emulate_default_handler(signal_hook::consts::signal::SIGTSTP).unwrap();
                //low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
            }
            _ => {
                error!("Not implemented: {:?}", c);
            }
        }
        self
    }
}

use std::panic;
use std::io::Error;
//use std::sync::Arc;
use std::sync::atomic::AtomicBool;
//use std::sync::atomic::{AtomicBool, Ordering};

use signal_hook::consts::signal::*;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
// A friend of the Signals iterator, but can be customized by what we want yielded about each
// signal.
use signal_hook::iterator::SignalsInfo;
//use signal_hook::iterator::exfiltrator::WithOrigin;
//use signal_hook::low_level;


fn event_loop(editor: &mut Editor) {
    let term_now = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        // When terminated by a second term signal, exit with exit code 1.
        // This will do nothing the first time (because term_now is false).
        //flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now)).unwrap();
        // But this will "arm" the above for the second time, by setting it to true.
        // The order of registering these is important, if you put this one first, it will
        // first arm and then terminate ‒ all in the first round.
        //flag::register(*sig, Arc::clone(&term_now)).unwrap();
    }
    let mut sigs = vec![
        // Some terminal handling
        //SIGTSTP,
        SIGCONT, SIGWINCH,
        // Reload of configuration for daemons ‒ um, is this example for a TUI app or a daemon
        // O:-)? You choose...
        SIGHUP,
        // Application-specific action, to print some statistics.
        SIGUSR1,
    ];
    sigs.extend(TERM_SIGNALS);
    let mut signals = signal_hook::iterator::Signals::new(&sigs).unwrap();

    //let (tx, rx) = channel::unbounded();
    let tx = g_app.tx.clone();
    let rx = g_app.rx.clone();

    {
        let tx = tx.clone();
        panic::set_hook(Box::new(move |w| {
            let mut t = Terminal::default();
            t.cleanup();
            //tx.send(Command::Quit).unwrap();
            //terminal_cleanup();
            info!("Custom panic hook: {:?}", w);
            info!("{:?}", backtrace::Backtrace::new());
        }));
    }


    let tx = tx.clone();
    thread::scope(|s| {
        //let tx = tx.clone();
        // user-mode
        s.spawn(|_| {
            let rx = rx.clone();
            let tx = tx.clone();
            let tx_background = g_background.tx.clone();
            let rx_background = g_background.rx.clone();
            //main_thread(editor, tx, rx, tx_background, rx_background);
            display_thread(editor, tx, rx, tx_background, rx_background);
            //low_level::emulate_default_handler(signal_hook::consts::signal::SIGUSR1).unwrap();
            low_level::raise(signal_hook::consts::signal::SIGUSR1).unwrap();
        });

        let tx2 = tx.clone();
        s.spawn(|_| signal_thread(tx2, &mut signals));

        s.spawn(|_| {
            let rx = rx.clone();
            let tx = tx.clone();
            let tx_background = g_background.tx.clone();
            let rx_background = g_background.rx.clone();
            //main_thread(editor, tx, rx, tx_background, rx_background);
            input_thread(tx, rx, tx_background, rx_background);
        });

        //let tx = tx_background.clone();
        (0..3).for_each(|i| {
            let i = i.clone();
            let tx_background = g_background.tx.clone();
            let rx_background = g_background.rx.clone();
            // save thread
            s.spawn(move |_| {
                info!("background thread {} start", i);
                background_thread(tx_background, rx_background);
                info!("background thread {} exit", i);
            });
        });

    }).unwrap();
    info!("exit main event loop");
}

fn display_thread(editor: &mut Editor,
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>) {
    let mut out = std::io::stdout();
    //editor.terminal.toggle();
    editor.command(&Command::Refresh);
    render_reset(&mut out);

    render_commands(&mut out, editor.clear().update().generate_commands());
    render_commands(&mut out, editor.clear().update().generate_commands());

    loop {
        channel::select! {
            recv(rx) -> c => {
                match c {
                    Ok(Command::Quit) => {
                        info!("display quit");
                        break;
                    }
                    Ok(c) => {
                        info!("display: {:?}", (c));
                        render_commands(&mut out, editor.command(&c).update().generate_commands());
                    }
                    Err(e) => {
                        info!("Error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
    editor.terminal.cleanup();
    info!("Display thread finished");
}

fn main_thread(editor: &mut Editor,
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>,
    ) {
    let mut q = Vec::new();
    let mut mode = Mode::Normal;
    let mut out = std::io::stdout();
    render_reset(&mut out);

    render_commands(&mut out, editor.clear().update().generate_commands());
    render_commands(&mut out, editor.clear().update().generate_commands());

    loop {
        let event = crossterm::event::read().unwrap();
        info!("Event {:?}", event);

        use std::convert::TryInto;
        let command: Result<Command, _> = event.try_into();
        // see if we got an immediate command
        match command {
            Ok(Command::Quit) => {
                info!("Command Quit");
                tx_background.send(Command::Quit).unwrap();
                return;
            }
            Ok(c) => {
                info!("Direct Command {:?}", c);
                render_commands(&mut out, editor.command(&c).update().generate_commands());
            }
            _ => ()
        }

        // parse user input
        match event.try_into() {
            Ok(e) => {
                use crate::bindings::parser::Elem;
                if let Elem::Control('r') = e {
                    info!("Refresh");
                    q.clear();
                    continue;
                }

                q.push(e);
                let result = mode.command()(q.as_slice());
                match result {
                    Ok((_, commands)) => {
                        for c in commands.iter() {
                            info!("Mode Command {:?}", c);
                            match c {
                                Command::Quit => {
                                    info!("Quit");
                                    tx_background.send(Command::Quit).unwrap();
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
                    Err(nom::Err::Incomplete(e)) => {
                        info!("Incomplete: {:?}\r", (&q, e));
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

struct AppChannel {
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>
}
impl Default for AppChannel {
    fn default() -> Self {
        let (tx, rx) = channel::unbounded();
        Self { tx, rx }
    }
}

lazy_static::lazy_static! {
    static ref g_app: AppChannel = AppChannel::default();
    static ref g_background: AppChannel = AppChannel::default();
}

use signal_hook::{iterator::Signals};
fn signal_thread(tx: channel::Sender<Command>, signals: &mut Signals) {
    use signal_hook::consts::signal::*;
    use signal_hook::consts::TERM_SIGNALS;
    use signal_hook::flag;

    let mut t = Terminal::default();
    let mut has_terminal = true;
    for info in signals {
        info!("Received a signal {:?}", info);
        match info {
            SIGCONT => {
                info!("signal continue {:?}", (has_terminal));
                //if !has_terminal {
                    //has_terminal = true;
                    t.enter_raw_mode();
                    tx.send(Command::Refresh).unwrap();
                //}
            }
            SIGWINCH => {
                tx.send(Command::Refresh).unwrap();
            }
            SIGTSTP => {
                info!("signal stop1 {:?}", (has_terminal));
                //if has_terminal {
                    has_terminal = false;
                    t.leave_raw_mode();
                    //tx.send(Command::Stop).unwrap();
                    //low_level::emulate_default_handler(SIGTSTP).unwrap();
                    //low_level::raise(SIGTSTP).unwrap();
                    low_level::raise(SIGSTOP).unwrap();
                //}
                info!("signal stop2 {:?}", (has_terminal));
            }
            SIGHUP => {
                info!("SIGHUP");
                break;
            }
            SIGUSR1 => {
                info!("SIGUSR1");
                //t.leave_raw_mode();
                //low_level::raise(SIGSTOP).unwrap();
                break;
            }
            _ => {
                info!("other sig {}", info);
                tx.send(Command::Quit).unwrap();
                break;
            }
        }
    }

    info!("signals thread exit");

    //let mut sigs = vec![SIGTSTP];
    ////let tx = tx.clone();
    //unsafe {
        //low_level::register(SIGTSTP, move || {
            //let mut t = Terminal::default();
            //t.cleanup();
            //tx.send(Command::Resume).unwrap();
            //info!("Received a stop signal");
            ////t.toggle();
            ////t.toggle();
            ////t.enter_raw_mode();
            ////t.cleanup();
            ////tx.send(Command::Stop).unwrap();
            ////terminal_cleanup();
        //}).unwrap();
    //}
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
                        // repeat until all threads have quit
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

use crate::cli::CliParams;
pub fn layout_cli(params: CliParams) {
    info!("paths: {:?}", (params.paths));

    let mut e = Editor::default();

    if params.paths.len() == 0 {
        e.add_window(FileBuffer::from_string(&"".into()));
    } else {
        use std::path::Path;
        params.paths.iter().for_each(|path| {
            if Path::new(&path).exists() {
                e.add_window(FileBuffer::from_path(&path.clone()));
            }
        });
    }
    event_loop(&mut e);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_1() {
        let mut e = Editor::default();
        let fb1 = FileBuffer::from_string(&"".to_string());
        let fb2 = FileBuffer::from_string(&"".to_string());
        e.add_window(fb1.clone());
        e.add_window(fb2.clone());
        e.add_window(fb2.clone());
        e.resize(100,20,0,0);

        use Command::*;
        let cs = vec![Insert('x'), BufferNext, Insert('y'), BufferNext, Insert('z')];
        cs.iter().for_each(|c| {
            e.command(c);
        });
        info!("A: {:?}", &fb1);
        info!("B: {:?}", &fb2);
        info!("C: {:?}", &mut e.layout.get_mut().generate_commands());
    }
}

