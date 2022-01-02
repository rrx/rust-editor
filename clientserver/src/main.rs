use futures::prelude::*;
use clap::{App, AppSettings, Arg, app_from_crate};
use failure;
use std::path::PathBuf;
use tempdir::TempDir;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use serde_json::json;
use tokio_util::codec;
use tokio_serde::formats::*;
use std::fs::File;
use tokio::signal;
use tokio::signal::unix::SignalKind;

async fn client_start(path: &PathBuf) -> Result<(), failure::Error> {
    use rustyline::*;
    use rustyline::error::*;
    log::info!("client start");

    let stream = UnixStream::connect(path).await?;

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();

    // Delimit frames using a length header
    let frame = codec::Framed::new(stream, codec::LengthDelimitedCodec::new());

    // Serialize frames with JSON
    let mut ser = tokio_serde::SymmetricallyFramed::new(frame, SymmetricalJson::default());

    let mut rl = Editor::<()>::with_config(config);
    loop {
        let p = format!("> ");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                ser.send(json!({"message": line})).await?;
                println!("Line: {}", line);
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
    rl.append_history("history.txt");

    Ok(())
}

fn client(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
        
    if foreground {
        let tmp_dir = TempDir::new("clientserver")?;
        let tmp_path = PathBuf::from(tmp_dir.path().join("daemon.pipe"));

        let (tx, rx) = mpsc::channel(1);
        // get listener
        let listener = rt.block_on(async {
            UnixListener::bind(&tmp_path)
        })?;

        // start server on a thread
        rt.spawn(async move {
            server_start(listener, true, rx)
        });

        // block on the client
        rt.block_on(client_start(&tmp_path))
    } else {
        if rt.block_on(UnixStream::connect(path)).is_err() {
            server_daemonize(path);
        }
        rt.block_on(client_start(path))
    }
}

async fn handler(stream: UnixStream) {
    let frame = codec::Framed::new(stream, codec::LengthDelimitedCodec::new());
    let mut ser = tokio_serde::SymmetricallyFramed::new(
        frame,
        SymmetricalJson::default(),
    );

    loop {
        while let Some(msg) = ser.try_next().await.unwrap() {
            log::info!("GOT: {:?}", msg);
            ser.send(json!({"awesome": true})).await;
        }
    }
    log::info!("handler exit")
}


async fn server_start(listener: UnixListener, foreground: bool, mut rx_exit: Receiver<()>) -> Result<(), failure::Error> {
    log::info!("server start");

    let mut hangup = tokio::signal::unix::signal(SignalKind::hangup()).unwrap();
    let mut quit = tokio::signal::unix::signal(SignalKind::quit()).unwrap();
    let mut terminate = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        log::info!("connection: {:?}", addr);
                        tokio::spawn(async move {
                            handler(stream).await;
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

            _ = rx_exit.recv() => {
                if foreground {
                    log::info!("exit");
                    break;
                }
            }
        }
    }
    Ok(())
}

fn server_daemonize(path: &PathBuf) -> Result<(), failure::Error> {
    log::info!("server daemonize");

    let mut out_path = path.clone();
    out_path.set_extension("out");

    let mut err_path = path.clone();
    err_path.set_extension("err");

    let stdout = File::create(out_path)?;
    let stderr = File::create(err_path)?;

    let pwd = std::env::current_dir()?;

    let d = daemonize::Daemonize::new()
        //.pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .working_directory(pwd)
        .umask(0o177)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .exit_action(|| println!("Executed before master process exits"));

    match d.start() {
        Ok(_) => {
            log::info!("Success, daemonized");
            Ok(())
        }
        Err(e) => {
            log::error!("Error: {}", e);
            Err(e.into())
        }
    }
}

fn server(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    if rt.block_on(UnixStream::connect(&path)).is_ok() {
        return Err(failure::format_err!(
            "refusing to start: another daemon is already running"
        ));
    }

    match std::fs::remove_file(&path) {
        Ok(_) => {}
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {}
            _ => {
                return Err(e.into());
            }
        },
    }

    unsafe { libc::umask(0o177); }

    if foreground {
        let result = rt.block_on(async {
            UnixListener::bind(&path)
        });
        let (tx, rx) = mpsc::channel(1);
        match result {
            Ok(listener) => {
                rt.block_on(server_start(listener, true, rx))
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                log::error!("Socket already in Use: {:?}", &path);
                Err(e.into())
            }
            Err(e) => {
                Err(e.into())
            }
        }
    } else {
        server_daemonize(path)
    }
}


fn main() -> Result<(), failure::Error> {
    env_logger::init();
    let matches = app_from_crate!()
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::new("socket_path")
                .short('s')
                .long("sock")
                .help("Path for socket")
                .takes_value(true)
                .default_value("/tmp/daemon.pipe")
        )
        .subcommand(
            App::new("client")
                .arg(
                    Arg::new("foreground")
                        .short('f')
                        .long("foreground")
                        .help("Run server in foreground")
                )
        )
        .subcommand(
            App::new("server")
                .arg(
                    Arg::new("foreground")
                        .short('f')
                        .long("foreground")
                        .help("Run server in foreground")
                )
        )
        .get_matches();

    let path = PathBuf::from(matches.value_of("socket_path").unwrap());

    match matches.subcommand() {
        Some(("client", client_matches)) => {
            client(&path, client_matches.is_present("foreground"))
        }
        Some(("server", server_matches)) => {
            server(&path, server_matches.is_present("foreground"))
        }
        _ => unreachable!()
    }
}

