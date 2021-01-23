use log::*;
use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;
use std::path::Path;

use crossbeam::thread;
use crossbeam::channel;
use std::convert::TryInto;
use std::sync::Arc;
use std::collections::VecDeque;

#[derive(Debug)]
enum Msg {
    Quit,
    Save(Buffer),
}

#[derive(Debug)]
struct Cursor {
    line_inx: usize,
    cx: usize, // char position relative to the line start
    rx: usize, // render position
    text: Arc<Rope>
}

impl Cursor {
}

#[derive(Debug)]
struct BufferList {
    buffers: VecDeque<Buffer>
}
impl Default for BufferList {
    fn default() -> Self {
        Self { buffers: VecDeque::new() }
    }
}

impl BufferList {
    fn get_mut(&mut self) -> &mut Buffer {
        self.buffers.iter_mut().next().unwrap()
    }
    fn get(&mut self) -> &Buffer {
        self.buffers.iter().next().unwrap()
    }
    fn add(&mut self, b: Buffer) {
        info!("Adding {:?}", b);
        self.buffers.push_front(b);
    }
    fn next(&mut self) {
        if let Some(b) = self.buffers.pop_front() {
            self.buffers.push_back(b);
        }
    }

    fn command(&mut self, c: &Command) {
        match c {
            Command::BufferNext => {
                self.next();
                info!("Next: {}", self.get().path);
            }
            _ => {
                self.get_mut().command(c);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Buffer {
    text: Rope,
    spec: Arc<ViewSpec>,
    path: String
}
impl Buffer {
    fn new(text: Rope, spec: Arc<ViewSpec>) -> Self {
        Self {text, spec, path: "".into()}
    }

    fn set_path(&mut self, path: &str) {
        self.path = String::from(path);
    }

    fn insert_char(&mut self, ch: char) {
        self.text.insert_char(0, ch);
    }

    fn save(&self) {
        let f = File::create(&self.path).unwrap();
        self.text.write_to(f).unwrap();
        info!("S: {:?}", (&self.text, &self.path));
        info!("Wrote: {} bytes to {}", self.text.len_bytes(), &self.path);
    }

    fn command(&mut self, c: &Command) {
        match c {
            Command::Insert(x) => {
                self.insert_char(*x);
            }
            _ => ()
        }
    }
}

fn event_loop(paths: Vec<String>, sx: u16, sy: u16) {
    log::info!("Start: {:?}", paths);


    let mut buffers = BufferList::default();

    if paths.len() == 0 {
        let mut buffer;
        let spec = Arc::new(ViewSpec::new(sx,sy,0,0));
        buffer = Buffer::new(Rope::from_str(""), spec);
        buffers.add(buffer);
    }

    paths.iter().for_each(|path| {
        if Path::new(&path).exists() {
            let spec = Arc::new(ViewSpec::new(sx,sy,0,0));
            let mut b = Buffer::new(Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap(), spec);
            b.set_path(&path);
            buffers.add(b);
        }
    });

    let (save_tx, save_rx) = channel::unbounded();
    let (render_tx, render_rx) = channel::unbounded();
    let (quit_tx, quit_rx) = channel::unbounded();

    thread::scope(|s| {

        // sub editor
        s.spawn(|_| {
            info!("sub-editor");
            // rope manipulation
            // when it's ready to save, clone the rope and send it to the save channel
            loop {
                channel::select! {
                    recv(quit_rx) -> _ => break,
                    recv(render_rx) -> r => {
                        match r {
                            Ok(c) => {
                                buffers.command(&c);
                                match c {
                                    Command::Save => {
                                        info!("Save");
                                        let b = buffers.get();
                                        save_tx.send(Msg::Save(b.clone())).unwrap();
                                    }
                                    _ => (),//info!("R: {:?}", c),
                                }
                            }
                            Err(e) => {
                                info!("E: {:?}", e);
                                return;
                            }
                        }
                    }
                }
            }
        });

        // user-mode
        s.spawn(|_| {
            let mut q = Vec::new();
            let mut mode = Mode::Normal;
            loop {
                let event = crossterm::event::read().unwrap();

                // see if we got an immediate command
                match event.try_into() {
                    Ok(Command::Quit) => {
                        info!("Quit");
                        for _ in 0..2 {
                            quit_tx.send(Msg::Quit).unwrap();
                        }
                        return;
                    }
                    Ok(Command::Save) => {
                        info!("Save");
                        render_tx.send(Command::Save).unwrap();
                        continue;
                    }
                    _ => ()
                }

                // parse user input
                match event.try_into() {
                    Ok(e) => {
                        q.push(e);
                        let result = mode.command()(q.as_slice());
                        match result {
                            Ok((_, Command::Quit)) => {
                                info!("Quit");
                                for _ in 0..2 {
                                    quit_tx.send(Msg::Quit).unwrap();
                                }
                                return;
                            }
                            Ok((_, Command::Mode(m))) => {
                                mode = m;
                                q.clear();
                            }
                            Ok((_, x)) => {
                                info!("[{:?}] Ok: {:?}\r", mode, (&q, &x));
                                q.clear();
                                render_tx.send(x).unwrap();
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
        });


        // save thread
        s.spawn(|_| {
            loop {
                channel::select! {
                    recv(quit_rx) -> _ => break,
                    recv(save_rx) -> c => {
                        match c {
                            Ok(Msg::Save(buffer)) => {
                                buffer.save();
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
            info!("save exit");
        });
    }).unwrap();
}

fn main() {
    use crossterm::*;
    use crossterm::terminal::*;
    use crossterm::event::*;
    let mut out = std::io::stdout();
    enable_raw_mode().unwrap();
    execute!(out, EnableMouseCapture).unwrap();

    let params = editor::cli::get_params();
    let (sx, sy) = crossterm::terminal::size().unwrap();
    info!("terminal: {:?}", (sx, sy));
    info!("paths: {:?}", (params.paths));
    event_loop(params.paths, sx, sy);

    execute!(out, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
}


