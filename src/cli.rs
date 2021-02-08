use clap::{crate_version, Arg};


pub struct CliParams {
    pub paths: Vec<String>,
    pub debug: bool,
}

pub fn get_params() -> CliParams {
    let matches = clap::App::new("editor")
        .version(crate_version!())
        .arg(
            Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .help("Silence all output"),
        )
        .arg(
            Arg::with_name("timestamp")
                .short("t")
                .help("prepend log lines with a timestamp")
                .takes_value(true)
                .possible_values(&["none", "sec", "ms", "ns"]),
        )
        .arg(Arg::with_name("d").short("d").help("Debug flag"))
        .arg(
            Arg::with_name("INPUT")
                .help("File to edit")
                .required(false)
                .multiple(true),
        )
        .get_matches();

    pretty_env_logger::init();

    // Get filepath from commandline
    let mut paths = Vec::new();
    match matches.values_of("INPUT") {
        Some(p) => paths.append(&mut p.map(|x| x.into()).collect::<Vec<String>>()),
        _ => (),
    }

    CliParams {
        paths,
        debug: matches.is_present("d"),
    }
}
