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

        // get listener
        let listener = rt.block_on(async {
            UnixListener::bind(&tmp_path)
        })?;

        // start server on a thread
        rt.spawn(async move {
            server_start(listener)
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

async fn server_start(listener: UnixListener) -> Result<(), failure::Error> {
    log::info!("server start");
    tokio::select! {
        Ok((stream, _)) = listener.accept() => {
            log::info!("connection");
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
        //.chown_pid_file(true)      // is optional, see `Daemonize` documentation
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
        
    if foreground {
        let listener = UnixListener::bind(path)?;
        rt.block_on(server_start(listener))
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

