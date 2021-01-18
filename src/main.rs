//extern crate ropey;
//extern crate termion;
//extern crate unicode_segmentation;
//extern crate clap;
//extern crate crossterm;
//extern crate stderrlog;

use log::*;
use clap::{Arg, App, crate_version};
use std::str::FromStr;

use editor::text::TextBuffer;

fn main() {
    let matches = App::new("editor")
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
            .required(true)
            .index(1))
        .get_matches();

    let verbose = matches.occurrences_of("verbosity") as usize;
    let quiet = matches.is_present("quiet");
    let ts = matches.value_of("timestamp").map(|v| {
        stderrlog::Timestamp::from_str(v).unwrap_or_else(|_| {
            clap::Error {
                message: "invalid value for 'timestamp'".into(),
                kind: clap::ErrorKind::InvalidValue,
                info: None,
            }.exit()
        })
    }).unwrap_or(stderrlog::Timestamp::Off);

    stderrlog::new()
        .module(module_path!())
        .quiet(quiet)
        .verbosity(verbose)
        .timestamp(ts)
        .init()
        .unwrap();


    // Get filepath from commandline
    let filepath = matches.value_of("INPUT").unwrap();
    let mut buf = TextBuffer::from_path(&filepath).unwrap();

    if matches.is_present("d") {
        editor::debug(&mut buf);
    } else {
        // set unbuffered
        editor::gui(&mut buf);
    }
    info!("End");
}


