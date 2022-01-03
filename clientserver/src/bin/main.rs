use clap::{App, AppSettings, Arg, app_from_crate};
use failure;
use std::path::PathBuf;

use clientserver::*;

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
                    Arg::new("detach")
                        .long("detach")
                        .help("Run detached, piping stdio to a log file")
                )
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
            server(&path, server_matches.is_present("foreground"), server_matches.is_present("detach"))
        }
        _ => unreachable!()
    }
}

