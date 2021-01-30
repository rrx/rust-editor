use clap::{Arg, crate_version};
use std::str::FromStr;

pub struct CliParams {
    pub paths: Vec<String>,
    pub debug: bool
}

pub fn get_params() -> CliParams {
    let matches = clap::App::new("editor")
        .version(crate_version!())
        .arg(Arg::with_name("verbosity")
             .short("v")
             .multiple(true)
             .help("Increase message verbosity"))
        .arg(Arg::with_name("quiet")
             .short("q")
             .help("Silence all output"))
        .arg(Arg::with_name("timestamp")
             .short("t")
             .help("prepend log lines with a timestamp")
             .takes_value(true)
             .possible_values(&["none", "sec", "ms", "ns"]))
        .arg(Arg::with_name("d")
            .short("d")
            .help("Debug flag"))
        .arg(Arg::with_name("INPUT")
            .help("File to edit")
            .required(false)
            .multiple(true))
        .get_matches();

    //let verbose = matches.occurrences_of("verbosity") as usize;
    //let quiet = matches.is_present("quiet");
    //let ts = matches.value_of("timestamp").map(|v| {
        //stderrlog::Timestamp::from_str(v).unwrap_or_else(|_| {
            //clap::Error {
                //message: "invalid value for 'timestamp'".into(),
                //kind: clap::ErrorKind::InvalidValue,
                //info: None,
            //}.exit()
        //})
    //}).unwrap_or(stderrlog::Timestamp::Off);

    //stderrlog::new()
        //.module(module_path!())
        //.quiet(quiet)
        //.verbosity(verbose)
        //.timestamp(ts)
        //.init()
        //.unwrap();
    use pretty_env_logger::env_logger::Target;

    //pretty_env_logger::formatted_builder()
        //.target(Target::Stderr)
        //.init();
    pretty_env_logger::init();

    // Get filepath from commandline
    let mut paths = Vec::new();
    match matches.values_of("INPUT") {
        Some(p) => paths.append(&mut p.map(|x| x.into()).collect::<Vec<String>>()),
        _ => ()
    }

    CliParams {
        paths: paths,
        debug: matches.is_present("d")
    }
}

pub fn cli_setup() {
    let params = get_params();
    let path = params.paths.first().unwrap();
    log::info!("Start: {}", path);

    if false {
        use crate::text::TextBuffer;
        let mut buf = TextBuffer::from_path(&path).unwrap();

        if params.debug {
            crate::debug(&mut buf);
        } else {
            // set unbuffered
            crate::gui(&mut buf);
        }
    } else {
        if params.debug {
            crate::text::debug(path);
        } else {
            // set unbuffered
            crate::text::raw(path);
        }
    }

    log::info!("End");
}



