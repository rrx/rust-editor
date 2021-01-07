extern crate ropey;
extern crate termion;
extern crate unicode_segmentation;
extern crate clap;
extern crate crossterm;

use clap::{Arg, App};

use editor::text::TextBuffer;

fn main() {
    let matches = App::new("editor")
        .version("0.1.0")
        .arg(Arg::with_name("d")
            .short("d")
            .help("Debug flag"))
        .arg(Arg::with_name("INPUT")
            .help("File to edit")
            .required(true)
            .index(1))
        .get_matches();

    // Get filepath from commandline
    let filepath = matches.value_of("INPUT").unwrap();
    let mut buf = TextBuffer::from_path(&filepath).unwrap();

    if matches.is_present("d") {
        editor::debug(&mut buf);
    } else {
        // set unbuffered
        editor::gui(&mut buf);
        println!("End");
    }
}


