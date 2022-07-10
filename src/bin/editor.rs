use clap;
use editor::cli;
use editor_core::ViewPos;
use editor_tui::{layout_cli, EditorConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::log_init()?;

    let params = cli::get_params();
    let (sx, sy) = crossterm::terminal::size().unwrap();

    layout_cli(
        &params.paths,
        EditorConfig {
            version: clap::crate_version!().to_string(),
        },
        ViewPos {
            w: sx as usize,
            h: sy as usize,
            x0: 0,
            y0: 0,
        },
    );

    Ok(())
}
