use futures::prelude::*;
use failure;
use std::path::PathBuf;
use tempfile;
use tokio::net::UnixStream;
use tokio_util::codec;
use tokio_serde::formats::*;
use std::os::unix::net::{UnixStream as StdUnixStream};

use crate::common::*;
use crate::server::*;

enum ClientCommand {
    Shutdown,
    Restart
}

/*
 * sometimes it takes a little bit for the server to become available
 * so we retry until it's able to connect
 */
pub async fn client_start(path: &PathBuf) -> Result<(), failure::Error> {
    loop {
        let stream = match UnixStream::connect(&path).await {
            Ok(stream) => stream,
            Err(err) => {
                log::error!("Error: {:?}", err);
                std::thread::sleep(std::time::Duration::from_millis(1000));
                continue;
            }
        };

        match client_start_once(stream).await {
            Ok(ClientCommand::Restart) => {
                log::info!("Restarting");
                continue;
            }
            _ => break

        }

    }
    log::info!("Client Exiting");
    Ok(())
}

/* readline client, that does RPC with the server */
async fn client_start_once(stream: UnixStream) -> Result<ClientCommand, failure::Error> {
    use rustyline::*;
    use rustyline::error::*;
    log::info!("client start");

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();

    // Delimit frames using a length header
    let frame = codec::Framed::new(stream, codec::LengthDelimitedCodec::new());
    // Serialize frames with JSON
    let mut ser = tokio_serde::SymmetricallyFramed::new(frame, SymmetricalCbor::default());

    let mut rl = Editor::<()>::with_config(config);
    loop {
        let p = format!("> ");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if line.len() > 0 {
                    log::info!("Line: {}", line);
                    let m = match line.as_str() {
                        "shutdown" => {
                            ser.send(Message::RequestServerShutdown).await?;
                            let result = ser.try_next().await?;
                            log::info!("result: {:?}", result);
                            return Ok(ClientCommand::Shutdown);
                        }
                        "restart" => {
                            ser.send(Message::RequestServerRestart).await?;
                            let result = ser.try_next().await?;
                            log::info!("result: {:?}", result);
                            return Ok(ClientCommand::Restart);
                        }
                        _ => Some(Message::TestRequest(line))
                    };

                    if let Some(msg) = m {
                        ser.send(msg).await?;
                        let result = ser.try_next().await?;
                        log::info!("result: {:?}", result);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("Encountered Eof");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.append_history("history.txt")?;

    Ok(ClientCommand::Shutdown)
}

pub fn client(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    if foreground {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        
        let tmp_dir = tempfile::tempdir()?;
        let tmp_path = PathBuf::from(tmp_dir.path().join("daemon.pipe"));

        // start server on a thread
        let server_path = tmp_path.clone();
        rt.spawn(async move {
            if let Err(err) = server_start(&server_path).await {
                log::error!("Error: {:?}", err);
            }
        });
       
        log::info!("client start on {:?}", &tmp_path);
        client_blocking(&tmp_path)
    } else {
        // try to connect, if you can't then daemonize
        let no_daemon = StdUnixStream::connect(path).is_err();

        if no_daemon {
            log::info!("spawning server");
            server_spawn(path)?;
        }

        client_blocking(path)

    }
}

/*
 * sync version of client_start
 */
fn client_blocking(path: &PathBuf) -> Result<(), failure::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(client_start(path))
}


