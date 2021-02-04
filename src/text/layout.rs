use log::*;
use ropey::Rope;
use super::*;
use std::sync::Arc;
use parking_lot::RwLock;
use std::ops::{Deref, DerefMut};
use crossbeam::thread;
use crossbeam::channel;
use signal_hook::low_level;
use std::io;
use std::fs::File;

#[derive(Debug)]
pub struct FileBuffer {
    pub text: Rope,
    pub path: String,
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

pub type LockedFileBuffer = Arc<RwLock<FileBuffer>>;

#[derive(Debug, Clone)]
pub struct BufferWindow {
    status: RenderBlock,
    left: RenderBlock,
    main: BufferBlock,
    w: usize, h: usize, x0: usize, y0: usize,
}

impl BufferWindow {
    fn new(buf: LockedFileBuffer) -> Self {
        Self {
            status: RenderBlock::default(),
            left: RenderBlock::default(),
            main: BufferBlock::new(buf),
            w:1, h:0, x0:0, y0:0,
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.status.clear();
        self.left.clear();
        self.main.clear();
        self
    }

    pub fn update(&mut self) -> &mut Self {
        //let fb = self.buf.read();

        //// refresh the cursors, which might contain stale data
        //self.start = cursor_update(&fb.text, self.main.w, &self.start);
        //self.cursor = cursor_update(&fb.text, self.main.w, &self.cursor);

        //// render the view, so we know how long the line is on screen
        //let (cx, cy, rows) = LineWorker::screen_from_cursor(
            //&fb.text, self.main.w, self.main.h, &self.start, &self.cursor);
        //// update start based on render
        //info!("buffer update: {:?}", (cx, cy, rows.len()));
        //let start = rows[0].cursor.clone();
        //self.start = start;
        //// update cursor position
        //self.rc.update(self.main.x0 + cx as usize, self.main.y0 + cy as usize);

        //// generate updates
        //let mut updates = rows.iter().map(|r| {
            //let mut u = RowUpdate::default();
            //u.item = RowUpdateType::Row(r.clone());
            //u
        //}).collect::<Vec<RowUpdate>>();
        //while updates.len() < self.main.h {
            //updates.push(RowUpdate::default());
        //}
        //self.main.update_rows(updates);

        self.main.update();

        let path = self.main.get_path();
        // update status
        let s = format!(
            "DEBUG: [{},{}] S:{} {} {:?}{:width$}",
            self.main.rc.cx, self.main.rc.cy,
            &self.main.start.simple_format(),
            path,
            (self.main.w, self.main.h, self.main.x0, self.main.y0),
            "",
            width=self.status.w);
        self.status.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]);

        // gutter
        let mut gutter = self.main.cache_render_rows.iter().enumerate().map(|(inx, row)| {
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

        //drop(fb);

        // update cache rows
        //self.cache_render_rows = rows;
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        out.append(&mut self.status.generate_commands());
        out.append(&mut self.left.generate_commands());
        out.append(&mut self.main.generate_commands());
        //out.append(&mut self.rc.generate_commands());
        out
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) -> &mut Self {
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;

        let prefix = 6;
        self.status.resize(w, 1, x0, y0 + h - 1);
        self.left.resize(prefix, h - 1, x0, y0);
        self.main.resize(w - prefix, h - 1, x0 + prefix, y0, 0);
        //let text = self.buf.read().text.clone();
        //self.cursor = cursor_resize(&text, w, &self.cursor);
        //self.start = cursor_resize(&text, w, &self.start);
        self.clear();
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
    command: BufferBlock,
    layout: WindowLayout,
    registers: Registers,
    highlight: String,
    w: usize, h: usize, x0: usize, y0: usize,
    terminal: Terminal
}
impl Default for Editor {
    fn default() -> Self {
        let mut layout = WindowLayout::default();
        Self {
            header: RenderBlock::default(),
            command: BufferBlock::new(FileBuffer::from_string(&"".to_string())),
            layout: layout,
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
        let text = b.main.get_text();
        let path = b.main.get_path();
        let cursor = &b.main.cursor;
        //let fb = b.buf.read();
        //let s = format!("Rust-Editor-{} {} {} Line:{}/{}{:width$}", clap::crate_version!(), fb.path, b.cursor.simple_format(), b.cursor.line_inx + 1, fb.text.len_lines(), width=b.w);
        let s = format!("Rust-Editor-{} {} {} Line:{}/{}{:width$}", clap::crate_version!(), path, cursor.simple_format(), cursor.line_inx + 1, text.len_lines(), width=b.main.w);
        //drop(fb);

        self.header.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Highlight, s))]);
        self.layout.get_mut().update();
        self.command.update();

        // render command line
        let line = self.command.get_text();
        self.command.left.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Normal, ">> ".to_string()))]);
        self.command.block.update_rows(vec![RowUpdate::from(LineFormat(LineFormatType::Normal, format!("{:width$}", line, width=self.command.block.w)))]);

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
        bufwin.main.set_focus(true);
        self.layout.add(bufwin);
    }

    pub fn resize(&mut self, w: usize, h: usize, x0: usize, y0: usize) {
        info!("Resize: {}/{}", w, h);
        self.w = w;
        self.h = h;
        self.x0 = x0;
        self.y0 = y0;
        self.header.resize(w, 1, x0, y0);
        self.layout.resize(w, h-2, x0, y0 + 1);
        self.command.resize(w, 1, x0, y0 + h - 1, 3);
    }

    pub fn get_command_line(&self) -> String {
        self.command.buf.read().text.line(0).to_string()
    }

    pub fn command_cancel(&mut self) -> &mut Self {
        self.command_reset()
    }

    pub fn command_exec(&mut self) -> &mut Self {
        let line = self.get_command_line();
        info!("EXEC: {}", line);
        // exec
        self.command_reset()
    }

    pub fn command_reset(&mut self) -> &mut Self {
        self.command.reset_buffer().update();
        self.command.set_focus(false);
        self.layout.get_mut().main.set_focus(true);
        self
    }

    pub fn command(&mut self, c: &Command) -> &mut Self {
        use crate::bindings::parser::Motion as M;
        use Command::*;
        match c {
            BufferNext => {
                self.layout.next().get_mut().clear().update();
                self.layout.get_mut().main.block.set_highlight(self.highlight.clone());
                //let fb = self.layout.buffers.get().buf.read();
                let path = self.layout.buffers.get().main.get_path();
                info!("Next: {}", path);
            }
            BufferPrev => {
                self.layout.prev().get_mut().clear().update();
                self.layout.get_mut().main.block.set_highlight(self.highlight.clone());
                //let fb = self.layout.get().buf.read();
                let path = self.layout.buffers.get().main.get_path();
                info!("Prev: {}", path);
            }
            Insert(x) => {
                self.layout.get_mut().main.insert_char(*x).update();
            }
            //Backspace => {
                //self.layout.get_mut().remove_range(-1).update();
            //}
            Join => {
                self.layout.get_mut().main.join_line().update();
            }
            Delete(reps, m) => {
                self.layout.get_mut().main.delete_motion(m, *reps).update();
            }
            Yank(reg, m) => {
                self.registers.update(reg, &self.layout.get_mut().main.motion_slice(m));
                self.update();
            }
            Paste(reps, reg, m) => {
                let s = self.registers.get(reg);
                self.layout.get_mut().main.paste_motion(m, &s, *reps).update();
            }
            RemoveChar(dx) => {
                self.layout.get_mut().main.remove_range(*dx).update();
            }
            Motion(reps, m) => {
                self.layout.get_mut().main.motion(m, *reps).update();
            }
            //CliInc(ch) => {
                //self.command.inc(*ch).update();
            //}
            CliEdit(cmds) => {
                self.command.set_focus(true);
                self.layout.get_mut().main.set_focus(false);
                for c in cmds {
                    self.command.command(&c);
                }
            }
            CliExec => {
                self.command_exec().update();
            }
            CliCancel => {
                self.command_cancel().update();
            }
            SearchInc(s) => {
                self.highlight = s.clone();
                self.layout.get_mut().main.clear().block.set_highlight(s.clone());
            }
            Search(s) => {
                self.highlight = s.clone();
                self.layout.get_mut().main.search(s.as_str()).search_next(0).update();
                self.layout.get_mut().main.clear().block.set_highlight(s.clone());
            }
            ScrollPage(ratio) => {
                let bw = self.layout.get();
                let xdy = bw.main.w as f32 / *ratio as f32;
                self.layout.get_mut().main.scroll(xdy as i32).update_from_start();
            }
            Scroll(dy) => {
                self.layout.get_mut().main.scroll(*dy as i32).update_from_start();
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.layout.get_mut().main.cursor_move_line(line_inx).update();
            }
            LineNav(dx) => {
                self.layout.get_mut().main.cursor_move_lc(*dx).update();
            }
            Resize(x, y) => {
                self.resize(*x as usize, *y as usize, self.x0, self.y0);
            }
            Mouse(x, y) => {
                let bw = self.layout.get_mut();
                match bw.main.cursor_from_xy(*x as usize, *y as usize) {
                    Some(c) => {
                        bw.main.cursor_move(c);//.update();
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
                self.command.set_focus(false);
                self.layout.get_mut().main.set_focus(true);
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

use signal_hook::consts::signal::*;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

//lazy_static::lazy_static! {
    //static ref READER: InputReader = InputReader::default();
//}

fn event_loop(editor: &mut Editor, reader: &mut InputReader) {
    use std::sync::atomic::AtomicBool;
    let term_now = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        // When terminated by a second term signal, exit with exit code 1.
        // This will do nothing the first time (because term_now is false).
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now)).unwrap();
        // But this will "arm" the above for the second time, by setting it to true.
        // The order of registering these is important, if you put this one first, it will
        // first arm and then terminate ‒ all in the first round.
        flag::register(*sig, Arc::clone(&term_now)).unwrap();
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

    // background channel
    let (tx_background, rx_background) = channel::unbounded();

    // handle panic
    use std::panic;
    panic::set_hook(Box::new(move |w| {
        let mut t = Terminal::default();
        t.cleanup();
        info!("Custom panic hook: {:?}", w);
        info!("{:?}", backtrace::Backtrace::new());
    }));

    let rx = reader.rx.clone();
    let tx = reader.tx.clone();
    thread::scope(|s| {
        // display
        s.spawn(|_| {
            let rx = rx.clone();
            let tx = tx.clone();
            let tx_background = tx_background.clone();
            let rx_background = rx_background.clone();
            display_thread(editor, tx, rx, tx_background, rx_background);

            // send a signal to trigger the signal thread to exit
            low_level::raise(signal_hook::consts::signal::SIGUSR1).unwrap();
        });

        // handle signals
        s.spawn(|_| signal_thread(tx.clone(), &mut signals));

        // user mode
        s.spawn(|_| input_thread(reader, tx_background.clone(), rx_background.clone()));

        (0..3).for_each(|i| {
            let i = i.clone();
            let tx_background = tx_background.clone();
            let rx_background = rx_background.clone();
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

fn display_thread(
    editor: &mut Editor,
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
                        tx_background.send(Command::Quit).unwrap();
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

//struct AppChannel {
    //tx: channel::Sender<Command>,
    //rx: channel::Receiver<Command>
//}
//impl Default for AppChannel {
    //fn default() -> Self {
        //let (tx, rx) = channel::unbounded();
        //Self { tx, rx }
    //}
//}

//lazy_static::lazy_static! {
    //static ref g_app: AppChannel = AppChannel::default();
    //static ref g_background: AppChannel = AppChannel::default();
//}

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
    let mut reader: InputReader = InputReader::default();

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
    event_loop(&mut e, &mut reader);
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

