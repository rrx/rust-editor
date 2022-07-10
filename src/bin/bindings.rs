use editor::cli;
use editor_bindings::InputReader;
use log::*;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::log_init()?;
    crossterm::terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    crossterm::execute!(out, crossterm::event::EnableMouseCapture)?;
    let mut reader = InputReader::default();
    loop {
        let event = crossterm::event::read().unwrap();
        match event.try_into() {
            Ok(e) => {
                let commands = reader.add(e);
                for command in commands {
                    info!("Command: {:?}", command);
                }
                if reader.is_quit() {
                    break;
                }
            }
            Err(err) => {
                info!("ERR: {:?}\r", (err));
            }
        }
    }
    crossterm::execute!(out, crossterm::event::DisableMouseCapture)?;
    crossterm::terminal::disable_raw_mode()?;
    out.flush()?;
    info!("\n\r");
    Ok(())
}
