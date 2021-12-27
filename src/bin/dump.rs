use editor_core::{Buffer, BufferConfig};
use std::env;
use editor_tui::format::*;
use editor_tui::string_to_elements;

fn main() -> std::result::Result<(), std::io::Error> {
    let paths: Vec<String> = env::args().skip(1).collect();
    println!("{:?}", paths);
    let config = BufferConfig::default();
    paths.iter().for_each(|path| {
        let fb = match Buffer::from_path(path) {
            Ok(b) => b,
            Err(err) => {
                println!("Error: {:?}", err);
                return;
            }
        };
        let text = fb.get_text();
        text.lines().for_each(|line| {
            let s = line.to_string().trim_end().to_string();
            println!("Line: {:?}", &s);
            let e = string_to_elements(&s, &config);
            println!("E:{:?}", (e));
            let r = format_wrapped(&s, 100, "".into(), &config);
            println!("F:{:?}", (r));
        });
    });
    Ok(())
}

