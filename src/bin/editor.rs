use clap;
use editor::cli;
use editor_bindings::InputReader;
use editor_core::{Buffer, ViewPos};
use editor_tui::{event_loop, Editor, EditorComplexLayout, EditorConfig};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::log_init()?;

    let params = cli::get_params();
    let (sx, sy) = crossterm::terminal::size().unwrap();

    let view = ViewPos {
        w: sx as usize,
        h: sy as usize,
        x0: 0,
        y0: 0,
    };
    let config = EditorConfig {
        version: clap::crate_version!().to_string(),
    };

    log::info!("paths: {:?}", (&params.paths));
    let mut reader: InputReader = InputReader::default();

    let mut layout = EditorComplexLayout::new(&config, view);

    if params.paths.len() == 0 {
        layout.add_window(Buffer::from_string(&"".into()));
    } else {
        params.paths.iter().for_each(|path| {
            if Path::new(&path).exists() {
                layout.add_window(Buffer::from_path_or_empty(&path.clone()));
            }
        });
    }
    let e = Editor::new(config, Box::new(layout));

    // event loop takes ownership of editor
    event_loop(e, &mut reader);

    Ok(())
}
