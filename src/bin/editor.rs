use clap;
use editor::cli;
use editor_tui::{layout_cli, EditorConfig};

fn main() -> std::io::Result<()> {
    let params = cli::get_params();
    layout_cli(
        &params.paths,
        EditorConfig {
            version: clap::crate_version!().to_string(),
        },
    );

    Ok(())
}
