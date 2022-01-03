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
    Restart,
    Continue,
    Error(String)
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

macro_rules! send {
    ($im:item, $ser:item) => {
        if let Err(e) = ser.send(m).await {
            return ClientCommand::Error(e.to_string());
        }
        match ser.try_next().await {
            Ok(result) => {
                log::info!("result: {:?} => {:?}", m, result);
                command
            }
            Err(e) => ClientCommand::Error(e.to_string())
        }
    }
}

struct CommandApp<'a>{
    app: clap::App<'a>,
    help: String

}
impl<'a> Default for CommandApp<'a> {
    fn default() -> Self {
        use clap::*;
        let mut app = App::new("command")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommand(App::new("shutdown").about("Shutdown the server"))
            .subcommand(App::new("restart").about("Restart the server"))
            .subcommand(
                App::new("start")
                    .about("Start processes")
                    .arg(
                        arg!(args: [ARGS]).multiple_occurrences(true).last(true)
                    )
            )
            .subcommand(
                App::new("stop")
                    .about("Stop processes")
                    .arg(
                        arg!(args: [ARGS]).takes_value(true).multiple_occurrences(true)
                    )
            )
            .subcommand(App::new("list").about("List processes"))
            ;
        let help = app.render_usage();
        Self { app, help }
    }
}

impl<'a> CommandApp<'a> {
    fn exec(&mut self, s: &str) -> Option<(Message, Option<ClientCommand>)> {
        use std::ffi::OsString;
        let mut args: Vec<OsString> = shlex::split(s).unwrap().iter().map(|x| OsString::from(x)).collect();
        args.insert(0, OsString::from("command"));
        let result = self.app.try_get_matches_from_mut(args);
        match result {
            Ok(matches) => {
                log::info!("matches: {:?}", matches);
                match matches.subcommand() {
                    Some(("shutdown", m)) => Some((Message::ServerShutdownReq, Some(ClientCommand::Shutdown))),
                    Some(("restart", m)) =>  Some((Message::ServerRestartReq, Some(ClientCommand::Restart))),
                    Some(("start", m)) => {
                        let mut args: Vec<String> = m.values_of("args").unwrap_or_default()
                            .map(|x| x.to_string())
                            .collect();
                        if args.len() > 0 {
                            let remaining = args.split_off(1);
                            Some((Message::ProcessStartReq(args.get(0).unwrap().into(), remaining), None))
                        } else {
                            None
                        }
                    }
                    Some(("list", m)) => Some((Message::ProcessListReq, None)),
                    Some(("stop", m)) => {
                        let args: Vec<String> = m.values_of("args").unwrap_or_default()
                            .map(|x| x.to_string())
                            .collect();//map(|vals| vals.collect());
                        Some((Message::ProcessStopReq(args), None))
                    }
                    _ => Some((Message::TestRequest(s.to_string()), None))
                }
            }
            Err(e) => {
                log::info!("err {:?}", e.to_string());
                None
            }
        }
    }
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
    // Serialize frames with Cbor
    let mut ser = tokio_serde::SymmetricallyFramed::new(frame, SymmetricalCbor::default());

    let mut commands = CommandApp::default();

    let mut rl = Editor::<()>::with_config(config);
    loop {
        let p = format!("> ");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                if line.len() > 0 {
                    log::info!("Line: {}", line);
                    //let message = parse_command(line.as_str(), &mut ser).await;
                    match commands.exec(line.as_str()) {
                        Some((req, maybe_command)) => {
                            // send a request, and expect a response
                            ser.send(req).await?;
                            let result = ser.try_next().await?;
                            log::info!("result: {:?}", result);
                            if let Some(command) = maybe_command {
                                return Ok(command);
                            }
                        }
                        None => ()
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


