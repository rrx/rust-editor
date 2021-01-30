use log::*;
use editor::text::*;
use ropey::Rope;
use std::io;
use std::fs::File;
use std::path::Path;

use crossbeam::thread;
use crossbeam::channel;
use std::convert::TryInto;

fn event_loop(paths: Vec<String>, sx: usize, sy: usize) {
    log::info!("Start: {:?}", paths);


    //let mut buffers = BufferList::default();
    let mut window = EditorWindow::new(sx, sy);

    if paths.len() == 0 {
        let buffer = Buffer::new(
            Rope::from_str(""),
            window.main.w, window.main.h, window.main.x0, window.main.y0);
        window.buffers.add(buffer);
    }

    paths.iter().for_each(|path| {
        if Path::new(&path).exists() {
            let mut b = Buffer::new(
                Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap(),
                window.main.w, window.main.h, window.main.x0, window.main.y0);
            b.set_path(&path);
            window.buffers.add(b);
        }
    });

    let (save_tx, save_rx) = channel::unbounded();
    let (quit_tx, quit_rx) = channel::unbounded();
    let window_app_tx = window.get_app_channel();

    thread::scope(|s| {
        // window
        s.spawn(|_| {
            info!("window");
            window.events(save_tx);
        });

        // user-mode
        s.spawn(|_| {
            let mut q = Vec::new();
            let mut mode = Mode::Normal;

            loop {
                let event = crossterm::event::read().unwrap();

                let command: Result<Command, _> = event.try_into();
                // see if we got an immediate command
                match command {
                    Ok(Command::Quit) => {
                        info!("Quit");
                        for _ in 0..2 {
                            quit_tx.send(Command::Quit).unwrap();
                            window_app_tx.send(Command::Quit).unwrap();
                        }
                        return;
                    }
                    //Ok(Command::Save) => {
                        //info!("Save");
                        //render_tx.send(Command::Save).unwrap();
                        //continue;
                    //}
                    Ok(c) => window_app_tx.send(c).unwrap(),
                    _ => ()
                }

                // parse user input
                match event.try_into() {
                    Ok(e) => {
                        q.push(e);
                        let result = mode.command()(q.as_slice());
                        //match result {
                            //Ok((_, Command::Quit)) => {
                                //info!("Quit");
                                //for _ in 0..2 {
                                    //quit_tx.send(Command::Quit).unwrap();
                                    //window_app_tx.send(Command::Quit).unwrap();
                                //}
                                //return;
                            //}
                            //Ok((_, Command::Mode(m))) => {
                                //mode = m;
                                //q.clear();
                            //}
                            //Ok((_, x)) => {
                                //info!("[{:?}] Ok: {:?}\r", mode, (&q, &x));
                                //q.clear();
                                //window_app_tx.send(x).unwrap();
                            //}
                            //Err(nom::Err::Incomplete(_)) => {
                                //info!("Incomplete: {:?}\r", (q));
                            //}
                            //Err(e) => {
                                //info!("Error: {:?}\r", (e, &q));
                                //q.clear();
                            //}
                        //}
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
                            Ok(Command::SaveBuffer(path, text)) => {
                                Buffer::save_text(&path, &text);
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
    layout_test();
}

fn main2() {
    layout_test();
    let params = editor::cli::get_params();
    use crossterm::*;
    use crossterm::terminal::*;
    use crossterm::event::*;
    let mut out = std::io::stdout();
    enable_raw_mode().unwrap();
    execute!(out, EnableMouseCapture).unwrap();
    execute!(out, DisableLineWrap).unwrap();

    let (sx, sy) = crossterm::terminal::size().unwrap();
    info!("terminal: {:?}", (sx, sy));
    info!("paths: {:?}", (params.paths));
    event_loop(params.paths, sx as usize, sy as usize);

    execute!(out, DisableMouseCapture).unwrap();
    execute!(out, EnableLineWrap).unwrap();
    disable_raw_mode().unwrap();
}


