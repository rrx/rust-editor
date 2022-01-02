use clap::{App, AppSettings, Arg, app_from_crate};
use failure;
use std::path::PathBuf;
use tempdir::TempDir;
use tokio::net::UnixStream;


async fn client_start(path: &PathBuf) -> Result<(), failure::Error> {
    let stream = UnixStream::connect(path).await?;
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
        server_start(&tmp_path);
        client_start(&tmp_path);

    } else {
        if rt.block_on(UnixStream::connect(path)).is_err() {
            server_start(path);
        }
        client_start(path);
    }

    Ok(())
}

fn server_start(path: &PathBuf) -> Result<(), failure::Error> {
    Ok(())
}

fn server_daemonize(path: &PathBuf) -> Result<(), failure::Error> {
    Ok(())
}

fn server(path: &PathBuf, foreground: bool) -> Result<(), failure::Error> {
    if foreground {
        server_start(path)
    } else {
        server_daemonize(path)
    }
}


fn main() -> Result<(), failure::Error> {
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

