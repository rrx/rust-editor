use super::*;
use crate::editor::Editor;
use crossbeam::channel;
use crossbeam::thread;
use editor_bindings::InputReader;
use editor_core::{Buffer, BufferConfig, Command, ViewPos};
use log::*;
use ropey::Rope;
use signal_hook::low_level;
use std::fs::File;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BufferWindow {
    pub status: RenderBlock,
    pub left: RenderBlock,
    pub main: BufferBlock,
    pub config: BufferConfig,
    pub view: ViewPos,
}

impl BufferWindow {
    pub fn new(buf: Buffer, view: ViewPos) -> Self {
        let main = BufferBlock::new(buf, view.clone());
        let config = main.get_config();
        Self {
            status: RenderBlock::new(view.clone()),
            left: RenderBlock::new(view.clone()),
            main,
            config,
            view,
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.status.clear();
        self.left.clear();
        self.main.clear();
        self
    }

    pub fn update(&mut self) -> &mut Self {
        self.main.update();

        let path = self.main.get_path();
        // update status
        let s = format!(
            "DEBUG: [{},{}] xh:{} S:{} {} {:?}{:width$}",
            self.main.rc.cx,
            self.main.rc.cy,
            self.main.cursor.x_hint,
            &self.main.cursor.simple_format(),
            path,
            (
                self.main.view.w,
                self.main.view.h,
                self.main.view.x0,
                self.main.view.y0
            ),
            "",
            width = self.status.view.w
        );
        self.status
            .update_rows(vec![RowUpdate::from(LineFormat::new(
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
                if row.wrap0 == 0 || inx == 0 {
                    line_display = row.line_inx + 1; // display one based
                }
                let fs;
                if line_display > 0 {
                    fs = format!(
                        "{:width$}\u{23A5}",
                        line_display,
                        width = self.left.view.w - 1
                    )
                } else {
                    fs = format!("{:width$}\u{23A5}", " ", width = self.left.view.w - 1)
                }
                RowUpdate::from(LineFormat::new(LineFormatType::Dim, fs))
            })
            .collect::<Vec<RowUpdate>>();
        while gutter.len() < self.left.view.h {
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

    pub fn resize(&mut self, view: ViewPos) -> &mut Self {
        self.view = view;

        let prefix = 6;
        self.status.resize(ViewPos {
            w: self.view.w,
            h: 1,
            x0: self.view.x0,
            y0: self.view.y0 + self.view.h - 1,
        });
        self.left.resize(ViewPos {
            w: prefix,
            h: self.view.h - 1,
            x0: self.view.x0,
            y0: self.view.y0,
        });
        let view = ViewPos {
            w: self.view.w - prefix,
            h: self.view.h - 1,
            x0: self.view.x0 + prefix,
            y0: self.view.y0,
        };

        self.main.resize(view, 0);
        self.clear();
        self
    }
}

pub struct WindowLayout {
    view: ViewPos,
    pub buffers: RotatingList<BufferWindow>,
}

impl WindowLayout {
    pub fn new(view: ViewPos) -> Self {
        Self {
            view: view.clone(),
            buffers: RotatingList::new(BufferWindow::new(Buffer::default(), view)),
        }
    }

    pub fn get_buffer(&mut self) -> &BufferWindow {
        let b = self.buffers.get_mut();
        b.resize(self.view.clone());
        b
    }

    pub fn get_buffer_mut(&mut self) -> &mut BufferWindow {
        let b = self.buffers.get_mut();
        b.resize(self.view.clone());
        b
    }

    pub fn resize(&mut self, view: ViewPos) {
        // each buffer needs to be resized on resize event
        // because each one caches things that depend on the size
        self.buffers.elements.iter_mut().for_each(|e| {
            e.resize(view.clone());
        });
    }

    pub fn clear(&mut self) -> &mut Self {
        self.buffers.get_mut().clear();
        self
    }
}

use signal_hook::consts::signal::*;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

pub fn event_loop(editor: Editor, reader: &mut InputReader) {
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
    let (tx, rx) = channel::unbounded();

    // handle panic
    use std::panic;
    panic::set_hook(Box::new(move |w| {
        let mut t = Terminal::default();
        t.cleanup();
        info!("Custom panic hook: {:?}", w);
        info!("{:?}", backtrace::Backtrace::new());
    }));

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
        s.spawn(|_| signal_thread(tx.clone(), tx_background.clone(), &mut signals));

        // user mode
        s.spawn(|_| {
            input_thread(
                reader,
                tx.clone(),
                tx_background.clone(),
                rx_background.clone(),
            )
        });

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
    mut editor: Editor,
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    _rx_background: channel::Receiver<Command>,
) {
    let mut terminal = Terminal::default();
    let mut out = std::io::stdout();
    terminal.enter_raw_mode();
    editor.command(&Command::Refresh);
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
                    Ok(c) => {
                        match c {
                            Command::SaveBuffer(_,_) => {
                                info!("background: {:?}", c);
                                tx_background.send(c).unwrap();
                            }
                            _ => {
                                info!("display: {:?}", (c));
                                editor.command(&c).iter().for_each(|x| {
                                    info!("sending: {:?}", (&x));
                                    tx.send(x.clone()).unwrap();
                                });
                                let commands = editor.update().generate_commands();
                                info!("commands: {:?}", (&commands));
                                render_commands(&mut out, commands);
                            }
                        }
                    }
                    Err(e) => {
                        info!("Error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
    terminal.cleanup();
    info!("Display thread finished");
}

use signal_hook::iterator::Signals;
fn signal_thread(
    tx: channel::Sender<Command>,
    tx_background: channel::Sender<Command>,
    signals: &mut Signals,
) {
    use signal_hook::consts::signal::*;

    let mut t = Terminal::default();
    let mut has_terminal = true;
    for info in signals {
        info!("Received a signal {:?}", info);
        match info {
            SIGCONT => {
                info!("signal continue {:?}", (has_terminal));
                t.enter_raw_mode();
                tx.send(Command::Refresh).unwrap();
            }
            SIGWINCH => {
                t.enter_raw_mode();
                tx.send(Command::Refresh).unwrap();
            }
            SIGTSTP => {
                info!("signal stop1 {:?}", (has_terminal));
                has_terminal = false;
                t.leave_raw_mode();
                low_level::raise(SIGSTOP).unwrap();
                info!("signal stop2 {:?}", (has_terminal));
            }
            SIGHUP => {
                info!("SIGHUP");
                break;
            }
            SIGUSR1 => {
                info!("SIGUSR1");
                break;
            }

            // panic
            SIGALRM => {
                info!("ALARM");
                tx.send(Command::Quit).unwrap();
                tx_background.send(Command::Quit).unwrap();
                break;
            }

            _ => {
                info!("other sig {}", info);
                tx_background.send(Command::Quit).unwrap();
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
