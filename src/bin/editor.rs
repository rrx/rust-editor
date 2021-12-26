use editor_tui::{cli, layout_cli};

fn main() -> std::io::Result<()> {
    let params = cli::get_params();
    layout_cli(params);
    Ok(())
}
