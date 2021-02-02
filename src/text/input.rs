use log::*;
use crossbeam::channel;
use super::*;
use futures::{future::FutureExt, select, StreamExt};

async fn input_thread_async(
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>,
    ) {
    let mut reader = crossterm::event::EventStream::new();
    let mut q = Vec::new();
    let mut mode = Mode::Normal;

    loop {
        let mut event = reader.next().fuse();

        select! {
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        use std::convert::TryInto;
                        let command: Result<Command, _> = event.try_into();
                        // see if we got an immediate command
                        match command {
                            Ok(Command::Quit) => {
                                info!("Command Quit");
                                tx_background.send(Command::Quit).unwrap();
                                tx.send(Command::Quit).unwrap();
                                return;
                            }
                            Ok(c) => {
                                info!("Direct Command {:?}", c);
                                tx.send(c).unwrap();
                            }
                            _ => ()
                        }
                        //
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
                                                    tx.send(Command::Quit).unwrap();
                                                    return;
                                                }
                                                Command::Mode(m) => {
                                                    mode = *m;
                                                    tx.send(Command::Mode(mode)).unwrap();
                                                    q.clear();
                                                }
                                                _ => {
                                                    info!("[{:?}] Ok: {:?}\r", mode, (&q, &c));
                                                    q.clear();
                                                    tx.send(c.clone()).unwrap();
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
                    Some(Err(e)) => {
                        info!("ERR: {:?}\r", (e));
                    }
                    None => break
                }
            }
        }
    }
    info!("input thread exit");
}

pub fn input_thread2(
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>) {
    async_std::task::block_on(input_thread_async(tx, rx, tx_background, rx_background));
}

use std::{io::stdout, time::Duration};
//use crossterm::
    //cursor::position,
    //event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    //execute,
    //terminal::{disable_raw_mode, enable_raw_mode},
    //Result,
//};

use crossterm::event::{poll, read};

use std::convert::TryInto;
pub fn input_thread(
    tx: channel::Sender<Command>,
    rx: channel::Receiver<Command>,
    tx_background: channel::Sender<Command>,
    rx_background: channel::Receiver<Command>,
    ) {

    let mut q = Vec::new();
    let mut mode = Mode::Normal;

    loop {
        match poll(Duration::from_millis(100)) {
            Ok(true) => {
                let event = crossterm::event::read().unwrap();
                info!("Event {:?}", event);

                let command: Result<Command, _> = event.try_into();
                // see if we got an immediate command
                match command {
                    Ok(Command::Quit) => {
                        info!("Command Quit");
                        tx_background.send(Command::Quit).unwrap();
                        tx.send(Command::Quit).unwrap();
                        return;
                    }
                    Ok(c) => {
                        info!("Direct Command {:?}", c);
                        tx.send(c).unwrap();
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
                            //tx.send(Command::Resume).unwrap();
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
                                            tx.send(Command::Quit).unwrap();
                                            return;
                                        }
                                        Command::Mode(m) => {
                                            mode = *m;
                                            tx.send(Command::Mode(mode)).unwrap();
                                            q.clear();
                                        }
                                        _ => {
                                            info!("[{:?}] Ok: {:?}\r", mode, (&q, &c));
                                            q.clear();
                                            tx.send(c.clone()).unwrap();
                                            //render_commands(&mut out, editor.command(&c).update().generate_commands());
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
            Ok(false) => {
                //info!("timeout");
                match rx_background.try_recv() {
                    Ok(Command::Quit) => {
                        info!("input quit");
                        tx_background.send(Command::Quit).unwrap();
                        return;
                    }
                    _ => ()
                }
            }
            Err(err) => {
                info!("ERR: {:?}\r", (err));
            }
        }
    }
    info!("Input thread finished");
}

