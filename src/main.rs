fn main() -> std::io::Result<()> {
    let params = editor::cli::get_params();
    use editor::text::layout_cli;
    layout_cli(params);
    Ok(())
}
