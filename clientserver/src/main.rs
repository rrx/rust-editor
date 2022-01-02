use futures::prelude::*;
use clap::{App, AppSettings, Arg, app_from_crate};
use failure;
use std::path::PathBuf;
//use tempdir::TempDir;
use tempfile;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use serde_json::json;
use tokio_util::codec;
use tokio_serde::formats::*;
use std::fs::File;
use tokio::signal;
use tokio::signal::unix::SignalKind;

async fn client_start(stream: UnixStream) -> Result<(), failure::Error> {
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
    let mut ser = tokio_serde::SymmetricallyFramed::new(frame, SymmetricalJson::default());

    let mut rl = Editor::<()>::with_config(config);
    loop {
        let p = format!("> ");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                log::info!("Line: {}", line);
                ser.send(json!({"message": line})).await?;
                let result = ser.try_next().await?;
                log::info!("result: {:?}", result);
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

    Ok(())
}

fn client(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
        
    if foreground {
        let tmp_dir = tempfile::tempdir()?;//TempDir::new("clientserver")?;
        let tmp_path = PathBuf::from(tmp_dir.path().join("daemon.pipe"));

        let (tx, rx) = mpsc::channel(1);

        // start server on a thread
        let server_path = tmp_path.clone();
        rt.spawn(async move {
            if let Err(err) = server_start(&server_path, true, rx).await {
                log::error!("Error: {:?}", err);
            }
        });
       
        log::info!("client start on {:?}", &tmp_path);
        loop {
            match rt.block_on(UnixStream::connect(&tmp_path)) {
                Ok(stream) => {
                    // block on the client
                    return rt.block_on(client_start(stream));
                }
                Err(err) => {
                    log::error!("Error: {:?}", err);
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }
        }


    } else {
        //use nix::{sys::wait::waitpid,unistd::{fork, ForkResult, write}};

        // try to connect, if you can't then daemonize
        let no_daemon = rt.block_on(UnixStream::connect(path)).is_err();

        if no_daemon {
            log::info!("spawning server");
            //match unsafe { fork() } {
                //Ok(ForkResult::Parent { child, .. }) => {
                    //log::info!("Continuing execution in parent process, new child has pid: {}", child);
                    //waitpid(child, None).unwrap();
                //}
                //Ok(ForkResult::Child) => {
                    //// Unsafe to use `println!` (or `unwrap`) here. See Safety.
                    //write(libc::STDOUT_FILENO, "I'm a new child process\n".as_bytes()).ok();
                    //server_daemonize(path)?;
                    ////unsafe { libc::_exit(0) };
                //}
                //Err(_) => println!("Fork failed"),
            //}
            server_daemonize(path)
        } else {
            let stream = rt.block_on(UnixStream::connect(path))?;
            rt.block_on(client_start(stream))
        }

    }
}

async fn handler(stream: UnixStream) {
    let frame = codec::Framed::new(stream, codec::LengthDelimitedCodec::new());
    let mut ser = tokio_serde::SymmetricallyFramed::new(
        frame,
        SymmetricalJson::default(),
    );

    loop {
        tokio::select! {
            result = ser.try_next() => {
                match result {
                    Ok(Some(msg)) => {
                        log::info!("GOT: {:?}", msg);
                        match ser.send(json!({"awesome": true})).await {
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
    log::info!("handler exit")
}


async fn server_start(path: &PathBuf, foreground: bool, mut rx_exit: Receiver<()>) -> Result<(), failure::Error> {
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

    if let Ok(listener) = result {
        loop {
            log::info!("Waiting");
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
                    log::info!("exit");
                    if foreground {
                        break;
                    }
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

    let daemon_path = path.clone();
    let d = daemonize::Daemonize::new()
        //.pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .working_directory(pwd)
        //.umask(0o177)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .exit_action(|| {
            log::info!("Executed before master process exits");
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            //let stream = rt.block_on(UnixStream::connect(daemon_path)).unwrap();
            //client_start(stream);
        });

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

    //unsafe { libc::umask(0o177); }

    let (tx, rx) = mpsc::channel(1);
    //let result = rt.block_on(async {
        //UnixListener::bind(&path)
    //});
    //match result {
        //Ok(listener) => {
            if foreground {
                rt.block_on(server_start(&path, true, rx))?;
            } else {
                server_daemonize(path)?;
                rt.block_on(server_start(&path, false, rx))?;
            }
            std::fs::remove_file(&path)?;
            log::info!("server exit");
            Ok(())
        //}
        //Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            //log::error!("Socket already in Use: {:?}", &path);
            //Err(e.into())
        //}
        //Err(e) => {
            //Err(e.into())
        //}
    //}
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

