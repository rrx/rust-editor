use log::*;
use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crossbeam::thread;
use crossbeam::channel;
use std::convert::TryInto;

#[derive(Debug)]
enum Msg {
    Quit,
    Save(Rope, String),
    Command(Command)
}

fn event_loop(path: &str) {
    log::info!("Start: {}", path);

    let mut text;
    if Path::new(path).exists() {
        text = Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap();
    } else {
        text = Rope::from_str("");
    }

    let (save_tx, save_rx) = channel::unbounded();
    let (render_tx, render_rx) = channel::unbounded();
    let (quit_tx, quit_rx) = channel::unbounded();

    thread::scope(|s| {
        s.spawn(|_| {
            info!("render");
            loop {
                channel::select! {
                    recv(quit_rx) -> _ => break,
                    recv(render_rx) -> c => {
                        match c {
                            Ok(Command::Insert(x)) => {
                                text.insert_char(0, x);
                                save_tx.send(Msg::Save(text.clone(), path.into())).unwrap();
                                continue;
                            }
                            Ok(Command::Save) => {
                                info!("Save");
                                save_tx.send(Msg::Save(text.clone(), path.into())).unwrap();
                            }
                            Ok(c) => info!("R: {:?}", c),
                            Err(e) => {
                                info!("E: {:?}", e);
                                return;
                            }
                        }
                    }
                }
            }
        });
        s.spawn(|_| {
            let (sx, sy) = crossterm::terminal::size().unwrap();
            info!("input: {:?}", (sx, sy));
            let mut q = Vec::new();
            let mut mode = Mode::Normal;
            loop {
                let event = crossterm::event::read().unwrap();
                // see if we got a command
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
                    }
                    _ => ()
                }

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
        s.spawn(|_| {
            loop {
                channel::select! {
                    recv(quit_rx) -> _ => break,
                    recv(save_rx) -> c => {
                        match c {
                            Ok(Msg::Save(text, path)) => {
                                info!("S: {:?}", (&text, &path));
                                let mut f = File::create(path).unwrap();
                                text.write_to(f).unwrap();
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
    let path = params.paths.first().unwrap();
    event_loop(path.as_str());

    execute!(out, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
}


