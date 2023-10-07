# Notes

Fixing UTF-8 handling
- https://unicode-rs.github.io/unicode-segmentation/unicode_segmentation/struct.GraphemeCursor.html

TTYs:
- http://www.linusakesson.net/programming/tty/index.php
- https://github.com/stemjail/tty-rs
- https://github.com/austinjones/tab-rs/
- https://meli.delivery/posts/2019-10-25-making-a-quick-and-dirty-terminal-emulator.html
- https://gist.github.com/Technius/43977937a28e8846d917b53605e32cc3
- https://www.reddit.com/r/rust/comments/bg7h8e/q_how_to_handle_io_of_a_subprocess_asynchronously/
- https://docs.rs/subprocess/0.2.6/subprocess/index.html
- https://docs.rs/pty/0.2.2/pty/
- https://github.com/wez/wezterm/blob/main/pty/examples/whoami_async.rs
- https://en.wikipedia.org/wiki/ANSI_escape_code#CSI_codes
- https://github.com/oconnor663/duct.rs
- https://github.com/pkgw/stund
- https://github.com/wez/wezterm/tree/main/pty
- http://www.rkoucha.fr/tech_corner/pty_pdip.html

Readline:
- https://github.com/dpc/async-readline

Glob search
- https://docs.rs/globset/0.4.6/globset/

Tabbing
- https://vim.fandom.com/wiki/Super_retab
- https://www.sublimetext.com/docs/3/indentation.html
- https://github.com/editorconfig/editorconfig/wiki/EditorConfig-Properties

Fuzzy search
- https://github.com/BurntSushi/fst
- https://github.com/andylokandy/simsearch-rs
- https://docs.rs/ngrammatic/0.3.2/ngrammatic
- https://github.com/lotabout/fuzzy-matcher
- https://github.com/Schlechtwetterfront/fuzzy-rs
- https://docs.rs/strsim/0.10.0/strsim/
-
terminal emulation
- https://github.com/wez/wezterm/tree/master/term/src
- https://github.com/alacritty/alacritty/tree/master/alacritty_terminal/src
- https://github.com/dflemstr/mux/tree/master/terminal-emulator
- https://github.com/ftilde/unsegen_terminal
- https://docs.rs/unsegen_pager/0.2.0/unsegen_pager/

Interesting Hobby editors
- https://github.com/mathphreak/mfte
- https://crates.io/crates/kiro-editor
- https://viewsourcecode.org/snaptoken/kilo/
- https://github.com/mathall/rim
- https://github.com/vamolessa/pepper
- https://amp.rs/
- https://github.com/gchp/iota
- https://github.com/mathphreak/mfte
- https://github.com/Kethku/neovide

Subprocess management
- https://github.com/hniksic/rust-subprocess

Text wrapping:
- https://github.com/mgeisler/textwrap
- https://github.com/ps1dr3x/easy_reader
- https://github.com/danielpclark/array_tool (Justification and string navigation)

Bling
- https://github.com/Phate6660/nixinfo

Scripting
- https://docs.mun-lang.org/

GUI
- https://github.com/tauri-apps/tauri

Rust Style and improvements
- https://github.com/JasonShin/fp-core.rs
- Iterator help: https://docs.rs/itertools/0.10.0/itertools/

EditorConfig
- https://github.com/mathphreak/mfte/commit/0787891f370a5ef66ee85351cab4468fc3fd518b
- https://crates.io/crates/editorconfig

handling signals in terminal:
- https://docs.rs/signal-hook/0.3.4/signal_hook/index.html
- https://github.com/Arkham/c_examples/blob/master/apue/signals/sigtstp.c


use ambassador to cleanup some delegated interfaces that are currently using deref inappropriately
- https://crates.io/crates/ambassador

Persistent data structures:
- https://github.com/orium/rpds

Remove abuse of unwrap:
- https://docs.rs/anyhow/1.0.38/anyhow/

Get some profiling going
- https://github.com/tikv/pprof-rs
- https://www.jibbow.com/posts/criterion-flamegraphs/

Vim Notes
- https://irian.to/blogs/introduction-to-vim-modes/


