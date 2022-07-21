use clap::{crate_version, Arg};
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};

pub struct CliParams {
    pub paths: Vec<String>,
    pub debug: bool,
}

pub fn log_init() -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder().build("output.log")?;
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Debug),
        )?;

    log4rs::init_config(config)?;
    Ok(())
}

pub fn get_params() -> CliParams {
    let matches = clap::App::new("editor")
        .version(crate_version!())
        .arg(
            Arg::with_name("verbosity")
                .short('v')
                .takes_value(true)
                .multiple_values(true)
                .multiple_occurrences(true)
                .help("Increase message verbosity"),
        )
        .arg(
            Arg::with_name("quiet")
                .short('q')
                .help("Silence all output"),
        )
        .arg(
            Arg::with_name("timestamp")
                .short('t')
                .help("prepend log lines with a timestamp")
                .takes_value(true)
                .possible_values(&["none", "sec", "ms", "ns"]),
        )
        .arg(Arg::with_name("d").short('d').help("Debug flag"))
        .arg(
            Arg::with_name("INPUT")
                .help("File to edit")
                .required(false)
                .takes_value(true)
                .multiple_values(true)
                .multiple_occurrences(true),
        )
        .get_matches();

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
