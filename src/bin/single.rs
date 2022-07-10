use editor::cli;
use editor_bindings::InputReader;
use editor_core::ViewPos;
use editor_tui::{event_loop, Editor, EditorConfig, EditorSimpleLayout};
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logfile = FileAppender::builder().build("output.log")?;
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

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
