use editor::bindings::Reader;
use log::*;
use std::io::prelude::*;

fn main() -> crossterm::Result<()> {
    pretty_env_logger::init();
    crossterm::terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    crossterm::execute!(out, crossterm::event::EnableMouseCapture)?;
    Reader::process().unwrap();
    crossterm::execute!(out, crossterm::event::DisableMouseCapture)?;
    crossterm::terminal::disable_raw_mode()?;
    out.flush()?;
    info!("\n\r");
    Ok(())
}


