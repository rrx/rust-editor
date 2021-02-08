use super::*;
use crossbeam::channel;
use crossbeam::thread;
use log::*;
use parking_lot::RwLock;
use ropey::Rope;
use signal_hook::low_level;
use std::fs::File;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct FileBuffer {
    pub text: Rope,
    pub path: String,
    pub config: BufferConfig,
    version: u64,
}

impl FileBuffer {
    pub fn from_path(path: &String) -> Arc<RwLock<Self>> {
        let maybe_f = File::open(&path.clone());

        let text = match maybe_f {
            Ok(f) => Rope::from_reader(&mut io::BufReader::new(f)).unwrap(),
            Err(_) => Rope::from_str(""),
        };

        let config = BufferConfig::config_for(Some(path));
        info!("Add window: {:?}", config);
        Arc::new(RwLock::new(FileBuffer {
            path: path.clone(),
            text,
            config,
            version: 0,
        }))
    }

    pub fn from_string(s: &String) -> LockedFileBuffer {
        let text = Rope::from_str(s);
        Arc::new(RwLock::new(FileBuffer {
            path: "".into(),
            config: BufferConfig::config_for(None),
            text,
            version: 0,
        }))
    }
}

pub type LockedFileBuffer = Arc<RwLock<FileBuffer>>;

#[derive(Debug, Clone)]
pub struct BufferWindow {
    pub status: RenderBlock,
    pub left: RenderBlock,
    pub main: BufferBlock,
    w: usize,
    h: usize,
    x0: usize,
    y0: usize,
}

impl BufferWindow {
    fn new(buf: LockedFileBuffer) -> Self {
        Self {
            status: RenderBlock::default(),
            left: RenderBlock::default(),
            main: BufferBlock::new(buf),
            w: 1,
            h: 0,
            x0: 0,
            y0: 0,
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
            self.main.rc.cx,
            self.main.rc.cy,
            &self.main.start.simple_format(),
            path,
            (self.main.w, self.main.h, self.main.x0, self.main.y0),
            "",
            width = self.status.w
        );
        self.status.update_rows(vec![RowUpdate::from(LineFormat(
            LineFormatType::Highlight,
            s,
        ))]);

        // gutter
        let mut gutter = self
            .main
            .cache_render_rows
            .iter()
            .enumerate()
            .map(|(inx, row)| {
                let mut line_display = 0; // zero means leave line blank
                if row.cursor.wrap0 == 0 || inx == 0 {
                    line_display = row.cursor.line_inx + 1; // display one based
                }
                let fs;
                if line_display > 0 {
                    fs = format!("{:width$}\u{23A5}", line_display, width = self.left.w - 1)
                } else {
                    fs = format!("{:width$}\u{23A5}", " ", width = self.left.w - 1)
                }
                RowUpdate::from(LineFormat(LineFormatType::Dim, fs))
            })
            .collect::<Vec<RowUpdate>>();
        while gutter.len() < self.left.h {
            gutter.push(RowUpdate::default());
        }
        self.left.update_rows(gutter);
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = Vec::new();
        out.append(&mut self.status.generate_commands());
        out.append(&mut self.left.generate_commands());
        out.append(&mut self.main.generate_commands());
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
    w: usize,
    h: usize,
    x0: usize,
    y0: usize,
    pub buffers: RotatingList<BufferWindow>,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            w: 10,
            h: 10,
            x0: 0,
            y0: 0,
            buffers: RotatingList::default(),
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

    pub fn clear(&mut self) -> &mut Self {
        self.get_mut().clear();
        self
    }
}

use signal_hook::consts::signal::*;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

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
        SIGHUP, // Application-specific action, to print some statistics.
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
    })
    .unwrap();
    info!("exit main event loop");
}

fn display_thread(
    editor: &mut Editor,
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    _rx_background: channel::Receiver<Command>,
) {
    let mut out = std::io::stdout();
    command(editor, &Command::Refresh);
    render_reset(&mut out);

    render_commands(&mut out, editor.clear().update().generate_commands());

    let ticker = channel::tick(std::time::Duration::from_millis(100));

    loop {
        channel::select! {
            recv(ticker) -> _ => {
                if editor.is_quit {
                    tx_background.send(Command::Quit).unwrap();
                    break;
                }
            }
            recv(rx) -> c => {
                match c {
                    //Ok(Command::Quit) => {
                        //info!("background: {:?}", c);
                        //tx_background.send(c.unwrap()).unwrap();
                        //break;
                    //}
                    Ok(Command::Save) => {
                        info!("background: {:?}", c);
                        let b = editor.layout.get();
                        let text = b.main.get_text();
                        let path = b.main.get_path();
                        let command = Command::SaveBuffer(path, text);
                        tx_background.send(command).unwrap();
                    }
                    Ok(c) => {
                        info!("display: {:?}", (c));
                        command(editor, &c).iter().for_each(|x| {
                            tx.send(x.clone()).unwrap();
                        });
                        let commands = editor.update().generate_commands();
                        render_commands(&mut out, commands);
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

use signal_hook::iterator::Signals;
fn signal_thread(tx: channel::Sender<Command>, signals: &mut Signals) {
    use signal_hook::consts::signal::*;

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
}

pub fn save_text(path: &String, text: &Rope) {
    let f = File::create(path).unwrap();
    text.write_to(f).unwrap();
    info!("Wrote: {} bytes to {}", text.len_bytes(), path);
}

fn background_thread(tx: channel::Sender<Command>, rx: channel::Receiver<Command>) {
    loop {
        channel::select! {
            recv(rx) -> c => {
                match c {
                    Ok(Command::SaveBuffer(path, text)) => {
                        save_text(&path, &text);
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
        params.paths.iter().for_each(|path| {
            if Path::new(&path).exists() {
                e.add_window(FileBuffer::from_path(&path.clone()));
            }
        });
    }
    event_loop(&mut e, &mut reader);
}
