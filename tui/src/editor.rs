use super::*;
use crate::layout::*;
use editor_bindings::command_parse;
use editor_core::{Buffer, Command, Registers, Variable, Variables, ViewPos};
use log::*;
use std::path::Path;

pub trait EditorLayout {
    fn clear(&mut self);
    fn update(&mut self);
    fn generate_commands(&mut self) -> Vec<DrawCommand>;
    fn resize(&mut self, view: ViewPos);
    fn command(&mut self, c: &Command) -> Vec<Command>;
    fn get_buffer(&mut self) -> &BufferBlock;
    fn get_buffer_mut(&mut self) -> &mut BufferBlock;
}

pub struct EditorSimpleLayout {
    pub layout: WindowLayout,
    view: ViewPos,
}
impl EditorSimpleLayout {
    pub fn new(view: ViewPos) -> Self {
        let layout = WindowLayout::new(view.clone());
        Self {
            layout: layout,
            view,
        }
    }
}

impl EditorLayout for EditorSimpleLayout {
    fn clear(&mut self) {
        self.layout.clear();
    }
    fn update(&mut self) {
        self.layout.get_buffer_mut().update();
    }
    fn generate_commands(&mut self) -> Vec<DrawCommand> {
        self.layout.get_buffer_mut().generate_commands()
    }
    fn resize(&mut self, view: ViewPos) {
        self.layout.resize(view);
    }
    fn get_buffer(&mut self) -> &BufferBlock {
        &self.layout.get_buffer().main
    }
    fn get_buffer_mut(&mut self) -> &mut BufferBlock {
        &mut self.layout.get_buffer_mut().main
    }

    fn command(&mut self, c: &Command) -> Vec<Command> {
        use Command::*;
        self.layout.get_buffer_mut().main.command(c);

        match c {
            Resize(x, y) => {
                let view = ViewPos {
                    w: *x as usize,
                    h: *y as usize,
                    x0: self.view.x0,
                    y0: self.view.y0,
                };
                self.resize(view);
                vec![]
            }
            _ => vec![],
        }
    }
}

pub struct EditorComplexLayout {
    header: RenderBlock,
    cmd_block: BufferBlock,
    pub layout: WindowLayout,
    highlight: String,
    view: ViewPos,
    version: String,
}

impl EditorComplexLayout {
    pub fn new(config: &EditorConfig, view: ViewPos) -> Self {
        let layout = WindowLayout::new(Self::layout_view(&view));
        Self {
            header: RenderBlock::new(Self::header_view(&view)),
            cmd_block: BufferBlock::new(
                Buffer::from_string(&"".to_string()),
                Self::cmd_view(&view),
            ),
            layout: layout,
            highlight: String::new(),
            view: view.clone(),
            version: config.version.clone(),
        }
    }

    pub fn update_cmd_normal(&mut self) -> &mut Self {
        self.cmd_block.update();
        // render command line
        let line = self.cmd_block.get_text();
        self.cmd_block
            .left
            .update_rows(vec![RowUpdate::from(LineFormat::new(
                LineFormatType::Normal,
                ">> ".to_string(),
            ))]);
        self.cmd_block
            .block
            .update_rows(vec![RowUpdate::from(LineFormat::new(
                LineFormatType::Normal,
                format!("{:width$}", line, width = self.cmd_block.block.view.w),
            ))]);
        self
    }

    pub fn add_window(&mut self, buf: Buffer) {
        let mut bufwin = BufferWindow::new(buf, Self::layout_view(&self.view));
        bufwin.main.set_focus(true);
        self.layout.buffers.add(bufwin);
    }

    pub fn get_command_line(&self) -> String {
        self.cmd_block.buf.get_text().line(0).to_string()
    }

    pub fn command_cancel(&mut self) -> &mut Self {
        self.command_reset()
    }

    pub fn command_update(&mut self) -> &mut Self {
        let line = self.get_command_line();
        if line.len() > 1 {
            let (first, last) = line.split_at(1);
            match first {
                "/" | "?" => {
                    self.highlight = last.to_string();
                    self.layout
                        .get_buffer_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                }
                _ => (),
            }
        } else {
            self.highlight.truncate(0);
        }
        self.update_cmd_normal();
        self
    }

    pub fn command_exec(&mut self) -> Vec<Command> {
        let line = self.get_command_line();
        info!("EXEC: {}", line);

        if line.len() > 1 {
            let (first, last) = line.split_at(1);
            match first {
                "/" => {
                    self.search_update(last.to_string());
                    self.layout
                        .get_buffer_mut()
                        .main
                        .search(last, false)
                        .search_next(0)
                        .update();
                    self.layout
                        .get_buffer_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                    self.command_reset();
                    vec![]
                }
                "?" => {
                    self.search_update(last.to_string());
                    self.layout
                        .get_buffer_mut()
                        .main
                        .search(last, true)
                        .search_next(0)
                        .update();
                    self.layout
                        .get_buffer_mut()
                        .main
                        .clear()
                        .block
                        .set_highlight(last.to_string());
                    self.command_reset();
                    vec![]
                }
                ":" => {
                    self.command_reset();
                    //self.cmd_block.generate_commands();
                    match command_parse(last) {
                        Ok(commands) => {
                            info!("command parse: {:?}", commands);
                            self.update();
                            commands
                        }
                        Err(err) => {
                            error!("command parse: {:?}", err);
                            self.command_output(&String::from("ERROR"));
                            self.update();
                            vec![]
                        }
                    }
                }
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    pub fn command_output(&mut self, s: &String) -> &mut Self {
        self.cmd_block
            .left
            .update_rows(vec![RowUpdate::from(LineFormat::new(
                LineFormatType::Normal,
                "OUT".to_string(),
            ))]);
        self.cmd_block
            .block
            .update_rows(vec![RowUpdate::from(LineFormat::new(
                LineFormatType::Bold,
                format!("{:width$}", s, width = self.cmd_block.block.view.w),
            ))]);
        self
    }

    pub fn command_reset(&mut self) -> &mut Self {
        self.cmd_block.reset_buffer().update();
        self.cmd_block.set_focus(false);
        self.layout.get_buffer_mut().main.set_focus(true);
        self.update_cmd_normal();
        self
    }

    pub fn search_update(&mut self, s: String) -> &mut Self {
        self.layout
            .get_buffer_mut()
            .main
            .block
            .set_highlight(s.to_string());
        self
    }

    fn header_view(view: &ViewPos) -> ViewPos {
        ViewPos {
            w: view.w,
            h: 1,
            x0: view.x0,
            y0: view.y0,
        }
    }

    fn layout_view(view: &ViewPos) -> ViewPos {
        ViewPos {
            w: view.w,
            h: view.h - 2,
            x0: view.x0,
            y0: view.y0 + 1,
        }
    }

    fn cmd_view(view: &ViewPos) -> ViewPos {
        ViewPos {
            w: view.w,
            h: 1,
            x0: view.x0,
            y0: view.y0 + view.h - 1,
        }
    }
}

impl EditorLayout for EditorComplexLayout {
    fn clear(&mut self) {
        self.header.clear();
        self.cmd_block.clear();
        self.layout.clear();
    }

    fn update(&mut self) {
        let b = self.layout.get_buffer();
        let text = b.main.get_text();
        let path = b.main.get_path();
        let cursor = &b.main.cursor;
        let s = format!(
            "Rust-Editor-{} {} {} Line:{}/{}{:width$}",
            self.version,
            path,
            cursor.simple_format(),
            cursor.line_inx + 1,
            text.len_lines(),
            width = b.main.view.w
        );

        self.header
            .update_rows(vec![RowUpdate::from(LineFormat::new(
                LineFormatType::Highlight,
                s,
            ))]);
        self.layout.get_buffer_mut().update();
    }

    fn generate_commands(&mut self) -> Vec<DrawCommand> {
        let mut out = self.layout.get_buffer_mut().generate_commands();
        out.append(&mut self.header.generate_commands());
        out.append(&mut self.cmd_block.generate_commands());
        out
    }

    fn resize(&mut self, view: ViewPos) {
        info!("Resize: {}/{}", view.w, view.h);
        self.header.resize(Self::header_view(&view));
        self.layout.resize(Self::layout_view(&view));
        self.cmd_block.resize(Self::cmd_view(&view), 3);
        self.view = view;
    }

    fn get_buffer(&mut self) -> &BufferBlock {
        &self.layout.get_buffer().main
    }

    fn get_buffer_mut(&mut self) -> &mut BufferBlock {
        &mut self.layout.get_buffer_mut().main
    }

    fn command(&mut self, c: &Command) -> Vec<Command> {
        use Command::*;
        self.layout.get_buffer_mut().main.command(c);
        match c {
            BufferNext => {
                self.layout.buffers.next().get_mut().clear().update();
                self.layout
                    .get_buffer_mut()
                    .main
                    .block
                    .set_highlight(self.highlight.clone());
                let path = self.layout.buffers.get().main.get_path();
                info!("Next: {}", path);
                vec![]
            }

            BufferPrev => {
                self.layout.buffers.prev().get_mut().clear().update();
                self.layout
                    .get_buffer_mut()
                    .main
                    .block
                    .set_highlight(self.highlight.clone());
                let path = self.layout.buffers.get().main.get_path();
                info!("Prev: {}", path);
                vec![]
            }

            CliEdit(cmds) => {
                self.cmd_block.set_focus(true);
                self.layout.get_buffer_mut().main.set_focus(false);
                for c in cmds {
                    self.cmd_block.command(&c);
                }
                self.command_update().update();
                vec![]
            }
            CliExec => {
                let commands = self.command_update().command_exec();
                self.update();
                commands
            }
            CliCancel => {
                self.command_cancel().update();
                vec![]
            }

            Resize(x, y) => {
                let view = ViewPos {
                    w: *x as usize,
                    h: *y as usize,
                    x0: self.view.x0,
                    y0: self.view.y0,
                };
                self.resize(view);
                vec![]
            }

            Open(filename) => {
                let path = Path::new(filename);
                match path.canonicalize() {
                    Ok(c_path) => {
                        let buf = Buffer::from_path_or_empty(&c_path.to_str().unwrap().to_string());
                        self.add_window(buf);
                    }
                    Err(err) => {
                        error!("Error opening file: {:?}", (filename, err));
                        let buf = Buffer::from_path_or_empty(&filename.to_string());
                        self.add_window(buf);
                    }
                }
                vec![]
            }

            VarGet(s) => {
                //let v = self.variables.get(&Variable(s.clone()));
                //self.command_output(&format!("get {} = {}", s, v)).update();
                vec![]
            }
            VarSet(a, b) => {
                //let k = Variable(a.clone());
                //let _v = self.variables.update(&k, b);
                //self.command_output(&format!("set {} = {}", a, b)).update();
                vec![]
            }

            _ => {
                vec![]
            }
        }
    }
}

pub struct Editor {
    config: EditorConfig,
    registers: Registers,
    variables: Variables,
    layout: Box<dyn EditorLayout + Send>,
    pub is_quit: bool,
}

pub struct EditorConfig {
    pub version: String,
}

impl Editor {
    pub fn new(config: EditorConfig, layout: Box<dyn EditorLayout + Send>) -> Self {
        Self {
            config,
            layout: layout,
            registers: Registers::default(),
            variables: Variables::default(),
            is_quit: false,
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.layout.clear();
        self
    }

    pub fn update(&mut self) -> &mut Self {
        self.layout.update();
        self
    }

    pub fn generate_commands(&mut self) -> Vec<DrawCommand> {
        self.layout.generate_commands()
    }

    pub fn resize(&mut self, view: ViewPos) {
        self.layout.resize(view);
    }

    pub fn command(&mut self, c: &Command) -> Vec<Command> {
        use Command::*;

        // pass the command to the layout
        self.layout.get_buffer_mut().command(c);

        match c {
            Join => {
                self.layout.get_buffer_mut().join_line().update();
                vec![]
            }
            Delete(reps, m) => {
                self.layout
                    .get_buffer_mut()
                    .delete_motion(m, *reps)
                    .update();
                vec![]
            }
            Yank(reg, m) => {
                self.registers
                    .update(reg, &self.layout.get_buffer_mut().motion_slice(m));
                self.update();
                vec![]
            }
            Paste(reps, reg, m) => {
                let s = self.registers.get(reg);
                self.layout
                    .get_buffer_mut()
                    .paste_motion(m, &s, *reps)
                    .update();
                vec![]
            }
            Motion(reps, m) => {
                self.layout.get_buffer_mut().motion(m, *reps).update();
                vec![]
            }
            ScrollPage(ratio) => {
                let block = self.layout.get_buffer();
                let xdy = block.view.w as f32 / *ratio as f32;
                self.layout
                    .get_buffer_mut()
                    .scroll(xdy as i32)
                    .update_from_start();
                vec![]
            }
            Scroll(dy) => {
                self.layout
                    .get_buffer_mut()
                    .scroll(*dy as i32)
                    .update_from_start();
                vec![]
            }
            Line(line_number) => {
                let line_inx = line_number - 1;
                self.layout
                    .get_buffer_mut()
                    .cursor_move_line(line_inx)
                    .update();
                vec![]
            }
            LineNav(dx) => {
                self.layout.get_buffer_mut().cursor_move_lc(*dx).update();
                vec![]
            }
            Mouse(x, y) => {
                let block = self.layout.get_buffer_mut();
                match block.cursor_from_xy(*x as usize, *y as usize) {
                    Some(c) => {
                        block.cursor_move(c);
                    }
                    _ => (),
                }
                vec![]
            }

            Quit => {
                info!("Quit");
                self.is_quit = true;
                vec![]
            }

            Save => {
                let block = self.layout.get_buffer();
                let text = block.get_text();
                let path = block.get_path();
                vec![SaveBuffer(path, text)]
            }

            SaveAs(filename) => {
                self.layout.get_buffer_mut().set_path(filename);
                vec![Save]
            }

            Refresh => {
                info!("Refresh");
                let (sx, sy) = crossterm::terminal::size().unwrap();
                let view = ViewPos {
                    w: sx as usize,
                    h: sy as usize,
                    x0: 0,
                    y0: 0,
                };
                self.resize(view);
                self.clear();
                self.update();
                //e.cmd_block.set_focus(false);
                self.layout.get_buffer_mut().set_focus(true);
                vec![]
            }

            Mode(_) => {
                vec![]
            }

            _ => {
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_view() -> ViewPos {
        ViewPos {
            w: 10,
            h: 10,
            x0: 0,
            y0: 0,
        }
    }

    fn test_config() -> EditorConfig {
        EditorConfig {
            version: "unknown".to_string(),
        }
    }

    #[test]
    fn test_layout_1() {
        let config = test_config();
        let mut layout = EditorComplexLayout::new(&config, test_view());
        let fb1 = Buffer::from_string(&"".to_string());
        let fb2 = Buffer::from_string(&"".to_string());
        layout.add_window(fb1.clone());
        layout.add_window(fb2.clone());
        layout.add_window(fb2.clone());

        let mut e = Editor::new(config, Box::new(layout));
        e.resize(ViewPos {
            w: 100,
            h: 20,
            x0: 0,
            y0: 0,
        });

        use Command::*;
        let cs = vec![
            Insert("x".to_string()),
            BufferNext,
            Insert("y".to_string()),
            BufferNext,
            Insert("z".to_string()),
        ];
        cs.iter().for_each(|c| {
            e.command(c);
        });
        info!("A: {:?}", &fb1);
        info!("B: {:?}", &fb2);
        let commands = e.layout.get_buffer_mut().generate_commands();
        info!("C: {:?}", &commands);
        println!("C: {:?}", &commands);
    }
}
