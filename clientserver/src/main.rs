use clap::{App, AppSettings, Arg, app_from_crate};
use failure;
use std::path::PathBuf;
use tempdir::TempDir;
use tokio::net::UnixStream;


async fn client_start(path: &PathBuf) -> Result<(), failure::Error> {
    use rustyline::*;
    use rustyline::error::*;

    let stream = UnixStream::connect(path).await?;

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();

    let mut rl = Editor::<()>::with_config(config);
    loop {
        let p = format!("> ");
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
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
        rt.spawn(server_start(tmp_path.clone()));
        client_start(&tmp_path);
    } else {
        if rt.block_on(UnixStream::connect(path)).is_err() {
            server_start(path.clone());
        }
        client_start(path);
    }

    Ok(())
}

async fn server_start(path: PathBuf) -> Result<(), failure::Error> {
    Ok(())
}

fn server_daemonize(path: &PathBuf) -> Result<(), failure::Error> {
    Ok(())
}

fn server(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
        
    if foreground {
        rt.block_on(server_start(path.clone()))
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

