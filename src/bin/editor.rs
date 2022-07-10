use clap;
use editor::cli;
use editor_core::ViewPos;
use editor_tui::{layout_cli, EditorConfig};
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
