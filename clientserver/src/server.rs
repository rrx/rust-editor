use futures::prelude::*;
use failure;
use std::path::PathBuf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Sender};
use tokio_util::codec;
use tokio_serde::formats::*;
use tokio::signal;
use tokio::signal::unix::SignalKind;
use std::os::unix::net::{UnixStream as StdUnixStream};

use crate::common::*;

enum ServerCommand {
    Shutdown
}

async fn handler(stream: UnixStream, tx_command: Sender<ServerCommand>) {
    let frame = codec::Framed::new(stream, codec::LengthDelimitedCodec::new());
    let mut ser = tokio_serde::SymmetricallyFramed::new(frame, SymmetricalCbor::default());

    loop {
        tokio::select! {
            result = ser.try_next() => {
                match result {
                    Ok(Some(msg)) => {
                        log::info!("GOT: {:?}", msg);
                        let m = match msg {
                            Message::RequestServerShutdown => {
                                log::info!("shutdown");
                                if ser.send(Message::ResponseServerShutdown).await.is_err() {
                                    log::error!("Unable to send response");
                                }
                                if tx_command.send(ServerCommand::Shutdown).await.is_err() {
                                    log::error!("Unable to send command");
                                }
                                return;
                            }
                            _ => Message::TestResponse
                        };

                        match ser.send(m).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!("send: {:?}", e);
                                break;
                            }
                        }
                    }

                    Ok(None) => {
                        log::info!("GOT: None");
                        break;
                    }

                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                        log::info!("connection reset");
                        break;
                    }

                    Err(e) => {
                        log::error!("Error: {:?}", e);
                        break;
                    }
                }
            }
        }
    }
    log::info!("handler exit");
}


pub async fn server_start(path: &PathBuf) -> Result<(), failure::Error> {
    log::info!("server start: {:?}", path);

    let mut hangup = tokio::signal::unix::signal(SignalKind::hangup())?;
    let mut quit = tokio::signal::unix::signal(SignalKind::quit())?;
    let mut terminate = tokio::signal::unix::signal(SignalKind::terminate())?;

    let result = UnixListener::bind(&path);
    match result {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            log::error!("Socket already in Use: {:?}", &path);
            return Err(e.into());
        }
        Err(e) => {
            return Err(e.into());
        }
    }

    let (tx_command, mut rx_command) = mpsc::channel(10);
    if let Ok(listener) = result {
        loop {
            log::info!("Waiting");
            let tx = tx_command.clone();
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            log::info!("connection: {:?}", addr);
                            tokio::spawn(async move {
                                handler(stream, tx).await;
                            });
                        }
                        Err(err) => {
                            log::error!("Accept: {:?}", err);
                            break;
                        }
                    }
                }

                _ = signal::ctrl_c() => {
                    log::info!("interrupt received");
                    break;
                }

                _ = hangup.recv() => {
                    log::info!("hangup received");
                    break;
                }

                _ = quit.recv() => {
                    log::info!("quit received");
                    break;
                }

                _ = terminate.recv() => {
                    log::info!("terminate received");
                    break;
                }
                
                cmd = rx_command.recv() => {
                    match cmd {
                        Some(ServerCommand::Shutdown) => {
                            log::info!("shutdown");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn server_spawn(path: &PathBuf) -> Result<(), failure::Error> {
    let args: Vec<String> = std::env::args().collect();
    let program = args.get(0).unwrap();
    let _child = std::process::Command::new(program)
        .args(["--sock", path.to_str().unwrap(), "server", "--detach"])
        .spawn()?;
    Ok(())
}

pub fn server(path: &PathBuf, foreground: bool, detach: bool) -> Result<(), failure::Error> {
    if StdUnixStream::connect(&path).is_ok() {
        return Err(failure::format_err!(
            "refusing to start: another daemon is already running: {:?}", &path
        ));
    }

    match std::fs::remove_file(&path) {
        Ok(_) => {
            log::info!("Removed stale socket: {:?}", path);
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {}
            _ => {
                return Err(e.into());
            }
        },
    }

    unsafe { libc::umask(0o177); }

    if detach {
        use stdio_override::*;
        let mut log_path = path.clone();
        log_path.set_extension("log");
        let _out_guard = StdoutOverride::override_file(&log_path)?;
        let _err_guard = StderrOverride::override_file(&log_path)?;
        let _in_guard = StdinOverride::override_file("/dev/null")?;
        server_daemon(&path)?;
    } else if foreground {
        server_daemon(&path)?;
    } else {
        log::info!("Forking server");
        server_spawn(&path)?;
    }

    std::fs::remove_file(&path)?;
    log::info!("server exit");
    Ok(())
}

fn server_daemon(path: &PathBuf) -> Result<(), failure::Error> {
    let mut log_path = path.clone();
    log_path.set_extension("log");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    log::info!("asdf");
    rt.block_on(server_start(&path))
}

