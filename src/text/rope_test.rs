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


    let mut buffers = BufferList::default();
    let mut window = EditorWindow::new(sx, sy);

    if paths.len() == 0 {
        //let spec = ViewSpec::new(sx,sy,0,0);
        let buffer = Buffer::new(
            Rope::from_str(""),
            window.main.w, window.main.h, window.main.x0, window.main.y0);
        buffers.add(buffer);
    }

    paths.iter().for_each(|path| {
        if Path::new(&path).exists() {
            //let spec = Arc::new(ViewSpec::new(sx,sy,0,0));
            //let spec = ViewSpec::new(sx,sy,0,0);
            let mut b = Buffer::new(
                Rope::from_reader(&mut io::BufReader::new(File::open(&path.clone()).unwrap())).unwrap(),
                window.main.w, window.main.h, window.main.x0, window.main.y0);
            b.set_path(&path);
            buffers.add(b);
        }
    });

    let (save_tx, save_rx) = channel::unbounded();
    let (render_tx, render_rx) = channel::unbounded();
    let (quit_tx, quit_rx) = channel::unbounded();
    let window_app_tx = window.get_app_channel();
    let window_tx = window.get_channel();

    thread::scope(|s| {
        // window
        s.spawn(|_| {
            info!("window");
            window.events();
        });

        // sub-editor
        s.spawn(|_| {
            info!("sub-editor");
            let mut out = std::io::stdout();
            let mut b = buffers.get_mut();
            b.update_view();
            b.send_updates(&window_tx);

            loop {
                channel::select! {
                    recv(quit_rx) -> _ => break,
                    recv(render_rx) -> r => {
                        match r {
                            Ok(Command::Save) => {
                                info!("Save");
                                let b = buffers.get();
                                save_tx.send(Msg::Save(b.clone())).unwrap();
                            }
                            Ok(c) => {
                                info!("Command: {:?}", c);
                                buffers.command(&c);
                                //buffers.get_mut().update_view();
                                let b = buffers.get();
                                b.send_updates(&window_tx);
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

        // sub editor
        //s.spawn(|_| {
            //return;
            //info!("sub-editor");
            //// rope manipulation
            //// when it's ready to save, clone the rope and send it to the save channel
            //let mut out = std::io::stdout();
            //render_reset(&mut out);

            //// initial refresh
            //let commands = buffers.get_mut().update_view();
            //render_commands(&mut out, commands);

            //loop {
                //channel::select! {
                    //recv(quit_rx) -> _ => break,
                    //recv(render_rx) -> r => {
                        //match r {
                            //Ok(c) => {
                                //buffers.command(&c);
                                //match c {
                                    //Command::Save => {
                                        //info!("Save");
                                        //let b = buffers.get();
                                        //save_tx.send(Msg::Save(b.clone())).unwrap();
                                    //}
                                    //_ => ()
                                //}
                                //let commands = buffers.get_mut().update_view();
                                //render_commands(&mut out, commands);
                            //}
                            //Err(e) => {
                                //info!("E: {:?}", e);
                                //return;
                            //}
                        //}
                    //}

                //}
            //}
        //});

        // user-mode
        s.spawn(|_| {
            let mut q = Vec::new();
            let mut mode = Mode::Normal;

            //let mut out = std::io::stdout();
            //let mut b = buffers.get_mut();
            //b.update_view();
            //window_tx.send(EditorWindowUpdate::Main(b.get_updates().clone())).unwrap();
            //window_tx.send(EditorWindowUpdate::Cursor(b.cx + b.x0, b.cy + b.y0)).unwrap();

            loop {
                let event = crossterm::event::read().unwrap();

                let command: Result<Command, _> = event.try_into();
                // see if we got an immediate command
                match command {
                    Ok(Command::Quit) => {
                        info!("Quit");
                        for _ in 0..2 {
                            quit_tx.send(Msg::Quit).unwrap();
                            window_app_tx.send(Msg::Quit).unwrap();
                        }
                        return;
                    }
                    Ok(Command::Save) => {
                        info!("Save");
                        render_tx.send(Command::Save).unwrap();
                        continue;
                    }
                    Ok(c) => render_tx.send(c).unwrap(),
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
                                    window_app_tx.send(Msg::Quit).unwrap();
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
    execute!(out, DisableLineWrap).unwrap();

    let params = editor::cli::get_params();
    let (sx, sy) = crossterm::terminal::size().unwrap();
    info!("terminal: {:?}", (sx, sy));
    info!("paths: {:?}", (params.paths));
    event_loop(params.paths, sx as usize, sy as usize);

    execute!(out, DisableMouseCapture).unwrap();
    execute!(out, EnableLineWrap).unwrap();
    disable_raw_mode().unwrap();
}


