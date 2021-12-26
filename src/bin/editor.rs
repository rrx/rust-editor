use editor_tui::{EditorConfig, layout_cli};
use editor::cli;
use clap;

fn main() -> std::io::Result<()> {
    let params = cli::get_params();
    layout_cli(&params.paths, EditorConfig { version: clap::crate_version!().to_string() });

    Ok(())
}
