use editor::cli;
use editor_bindings::InputReader;
use editor_core::ViewPos;
use editor_tui::{event_loop, Editor, EditorConfig, EditorSimpleLayout};

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

    let layout = EditorSimpleLayout::new(view);
    let e = Editor::new(config, Box::new(layout));

    event_loop(e, &mut reader);

    Ok(())
}
